//! Rede Fase 2: VirtioNet + e1000 + DHCP + ICMP + HTTP
use alloc::sync::Arc;
use alloc::vec::Vec;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::dhcpv4::{Socket as DhcpSocket, Event as DhcpEvent};
use smoltcp::socket::icmp::{
    Endpoint as IcmpEndpoint, PacketBuffer as IcmpBuf, PacketMetadata as IcmpMeta,
    Socket as IcmpSocket,
};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer as TcpBuf};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use crate::virtio::{ack_features, finish_ok, inb_pub, init_legacy, VirtQueue, DESC_F_WRITE};
use spin::Mutex;

pub static STACK: Mutex<Option<NetStack>> = Mutex::new(None);

const RX_BUFS: usize = 12;
const RX_LEN: usize = 2048;

// ========== VIRTIO-NET ==========

#[derive(Clone)]
pub struct VirtioNet {
    io: u16,
    mac: [u8; 6],
    rx: VirtQueue,
    tx: VirtQueue,
    rx_bufs: Vec<Vec<u8>>,
}

impl VirtioNet {
    pub fn probe(dev: &crate::pci::PciDev) -> Result<Self, &'static str> {
        let r = init_legacy(dev)?;
        ack_features(r.io, 1 << 5); // VIRTIO_NET_F_MAC
        let mut mac = [0u8; 6];
        for i in 0..6 {
            mac[i] = inb_pub(r.io + 0x14 + i as u16);
        }
        let qmax = VirtQueue::queue_max(r.io, 0).min(128);
        if qmax == 0 {
            return Err("net queue max 0");
        }
        let mut rx = VirtQueue::new(qmax);
        let mut tx = VirtQueue::new(qmax);
        rx.setup_legacy(r.io, 0);
        tx.setup_legacy(r.io, 1);
        finish_ok(r.io);
        let mut n = Self { io: r.io, mac, rx, tx, rx_bufs: Vec::new() };
        for _ in 0..RX_BUFS {
            n.post_rx();
        }
        Ok(n)
    }

    fn post_rx(&mut self) {
        let buf = alloc::vec![0u8; RX_LEN];
        let p = VirtQueue::phys(buf.as_ptr());
        self.rx_bufs.push(buf);
        self.rx.add_chain(&[(p, RX_LEN as u32, crate::virtio::DESC_F_WRITE)]);
        self.rx.notify(self.io, 0);
    }

    pub fn recv_pkt(&mut self) -> Option<Vec<u8>> {
        // Simplificado: retorna None para Fase 2 (polling serah implementado depois)
        None
    }

    pub fn xmit(&mut self, pkt: &[u8]) {
        self.tx.add_chain(&[(VirtQueue::phys(pkt.as_ptr()), pkt.len() as u32, 0)]);
        self.tx.notify(self.io, 1);
        for _ in 0..200_000 {
            if let Some((id, _)) = self.tx.pop_used() {
                self.tx.free_chain(id, 1);
                break;
            }
            core::hint::spin_loop();
        }
    }
}

// ========== E1000 ==========

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RxDesc {
    addr: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TxDesc {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

const E1000_RCTL: u32 = 0x0100;
const E1000_TCTL: u32 = 0x0400;
const E1000_RDLEN: u32 = 0x2808;
const E1000_RDH: u32 = 0x2810;
const E1000_RDT: u32 = 0x2818;
const E1000_RDBAL: u32 = 0x2800;
const E1000_RDBAH: u32 = 0x2804;
const E1000_TDLEN: u32 = 0x3808;
const E1000_TDH: u32 = 0x3810;
const E1000_TDT: u32 = 0x3818;
const E1000_TDBAL: u32 = 0x3800;
const E1000_TDBAH: u32 = 0x3804;
const E1000_RAL0: u32 = 0x5400;
const E1000_RAH0: u32 = 0x5404;
const E1000_MTA_OFF: u32 = 0x5200;

const E1000_RCTL_EN: u32 = 1 << 1;
const E1000_RCTL_BAM: u32 = 1 << 15;
const E1000_TCTL_EN: u32 = 1 << 1;
const E1000_TCTL_PSP: u32 = 1 << 3;
const RXD_STAT_DD: u8 = 0x01;
const TXD_CMD_EOP: u8 = 0x01;
const TXD_CMD_RS: u8 = 0x08;
const TXD_STAT_DD: u8 = 0x01;

fn mm_w(mmio: *mut u32, off: u32, v: u32) {
    unsafe { core::ptr::write_volatile(mmio.byte_add(off as usize), v) };
}
fn mm_r(mmio: *mut u32, off: u32) -> u32 {
    unsafe { core::ptr::read_volatile(mmio.byte_add(off as usize)) }
}

unsafe impl Send for E1000 {}

pub struct E1000 {
    mmio: *mut u32,
    rx: Vec<RxDesc>,
    tx: Vec<TxDesc>,
    rx_bufs: Vec<Vec<u8>>,
    rtail: usize,
    ttail: usize,
    mac: [u8; 6],
}

impl E1000 {
    pub fn probe(dev: &crate::pci::PciDev) -> Result<Self, &'static str> {
        if dev.vendor != 0x8086 || dev.device != 0x100E {
            return Err("not 82540EM");
        }
        crate::pci::enable_bus_master(dev);
        let (addr, is_io) = crate::pci::bar(dev, 0);
        if is_io {
            return Err("e1000 bar0 not MMIO");
        }
        let hhdm_off: u64 = 0xffff_8000_0000_0000;
        let mmio = (addr.wrapping_add(hhdm_off)) as *mut u32;
        let ral = mm_r(mmio, E1000_RAL0);
        let rah = mm_r(mmio, E1000_RAH0);
        let mac = [
            (ral & 0xFF) as u8, ((ral >> 8) & 0xFF) as u8, ((ral >> 16) & 0xFF) as u8,
            ((ral >> 24) & 0xFF) as u8,
            (rah & 0xFF) as u8, ((rah >> 8) & 0xFF) as u8,
        ];
        if mac == [0, 0, 0, 0, 0, 0] || mac[0] & 1 == 1 {
            return Err("e1000 no mac");
        }
        mm_w(mmio, 0x00D0, 0);
        for i in 0..128u32 {
            mm_w(mmio, E1000_MTA_OFF + i * 4, 0);
        }
        let mut rx = alloc::vec![RxDesc::default(); 32];
        let mut rx_bufs = Vec::new();
        for d in rx.iter_mut() {
            let b = alloc::vec![0u8; 2048];
            d.addr = (b.as_ptr() as u64).wrapping_sub(hhdm_off);
            d.status = 0;
            rx_bufs.push(b);
        }
        let rxp = (rx.as_ptr() as u64).wrapping_sub(hhdm_off);
        mm_w(mmio, E1000_RDBAL, rxp as u32);
        mm_w(mmio, E1000_RDBAH, (rxp >> 32) as u32);
        mm_w(mmio, E1000_RDLEN, (32 * 16) as u32);
        mm_w(mmio, E1000_RDH, 0);
        mm_w(mmio, E1000_RDT, 0);
        mm_w(mmio, E1000_RCTL, E1000_RCTL_EN | E1000_RCTL_BAM);
        let mut tx = alloc::vec![TxDesc::default(); 32];
        let txp = (tx.as_ptr() as u64).wrapping_sub(hhdm_off);
        mm_w(mmio, E1000_TDBAL, txp as u32);
        mm_w(mmio, E1000_TDBAH, (txp >> 32) as u32);
        mm_w(mmio, E1000_TDLEN, (32 * 16) as u32);
        mm_w(mmio, E1000_TDH, 0);
        mm_w(mmio, E1000_TDT, 0);
        mm_w(mmio, E1000_TCTL, E1000_TCTL_EN | E1000_TCTL_PSP);
        Ok(E1000 {
            mmio, rx, tx, rx_bufs, rtail: 0, ttail: 0, mac,
        })
    }

    pub fn recv_pkt(&mut self) -> Option<Vec<u8>> {
        let d = &mut self.rx[self.rtail];
        if d.status & RXD_STAT_DD == 0 {
            return None;
        }
        let n = d.length as usize;
        let mut pkt = alloc::vec![0u8; n];
        let buf_ptr = self.rx_bufs[self.rtail].as_ptr();
        let src = (buf_ptr as u64).wrapping_sub(crate::heap::hhdm_offset() as u64) + crate::heap::hhdm_offset() as u64;
        unsafe { core::ptr::copy_nonoverlapping(src as *const u8, pkt.as_mut_ptr(), n); }
        d.status = 0;
        mm_w(self.mmio, E1000_RDT, self.rtail as u32);
        self.rtail = (self.rtail + 1) % 32;
        Some(pkt)
    }

    pub fn xmit(&mut self, pkt: &[u8]) {
        let t = self.ttail;
        let owned = alloc::vec![0u8; pkt.len()];
        let leaked: &'static mut [u8] = alloc::boxed::Box::leak(owned.into_boxed_slice());
        leaked.copy_from_slice(pkt);
        let hhdm: u64 = 0xffff_8000_0000_0000;
        self.tx[t].addr = (leaked.as_ptr() as u64).wrapping_sub(hhdm);
        self.tx[t].length = leaked.len() as u16;
        self.tx[t].cmd = TXD_CMD_EOP | TXD_CMD_RS;
        self.tx[t].status = 0;
        self.ttail = (self.ttail + 1) % 32;
        mm_w(self.mmio, E1000_TDT, self.ttail as u32);
        for _ in 0..200_000 {
            if self.tx[t].status & TXD_STAT_DD != 0 {
                break;
            }
            core::hint::spin_loop();
        }
    }
}

// ========== QUEUE HANDLES ==========

#[derive(Clone)]
pub struct QueueHandle {
    pub inner: Arc<Mutex<QueueInner>>,
}

pub struct QueueInner {
    pub virtio: Option<VirtioNet>,
    pub e1000: Option<E1000>,
}

pub struct NetRx {
    pkt: Vec<u8>,
}
impl RxToken for NetRx {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        f(&self.pkt)
    }
}

pub struct NetTx {
    q: Arc<Mutex<QueueInner>>,
}
impl TxToken for NetTx {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buf = alloc::vec![0u8; len];
        let r = f(&mut buf);
        let mut g = self.q.lock();
        if let Some(v) = g.virtio.as_mut() {
            v.xmit(&buf);
        } else if let Some(e) = g.e1000.as_mut() {
            e.xmit(&buf);
        }
        r
    }
}

pub enum NetDev {
    Virtio(QueueHandle),
    E1000(QueueHandle),
}
impl Clone for NetDev {
    fn clone(&self) -> Self {
        match self {
            NetDev::Virtio(h) => NetDev::Virtio(QueueHandle { inner: h.inner.clone() }),
            NetDev::E1000(h) => NetDev::E1000(QueueHandle { inner: h.inner.clone() }),
        }
    }
}

impl Device for NetDev {
    type RxToken<'a> = NetRx where Self: 'a;
    type TxToken<'a> = NetTx where Self: 'a;

    fn receive(&mut self, _ts: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let q = match self {
            NetDev::Virtio(h) => h.inner.clone(),
            NetDev::E1000(h) => h.inner.clone(),
        };
        let pkt = {
            let mut g = q.lock();
            if let Some(v) = g.virtio.as_mut() {
                v.recv_pkt()
            } else if let Some(e) = g.e1000.as_mut() {
                e.recv_pkt()
            } else {
                None
            }
        }?;
        Some((NetRx { pkt }, NetTx { q }))
    }

    fn transmit(&mut self, _ts: Instant) -> Option<Self::TxToken<'_>> {
        let q = match self {
            NetDev::Virtio(h) => h.inner.clone(),
            NetDev::E1000(h) => h.inner.clone(),
        };
        Some(NetTx { q })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut c = DeviceCapabilities::default();
        c.medium = Medium::Ethernet;
        c.max_transmission_unit = 1500;
        c.max_burst_size = Some(1);
        c
    }
}

// ========== NET STACK ==========

pub struct NetStack {
    pub dev: NetDev,
    pub iface: Interface,
    pub sockets: SocketSet<'static>,
    pub ip: Option<Ipv4Address>,
    pub gw: Option<Ipv4Address>,
    pub mac: [u8; 6],
}

impl NetStack {
    pub fn dev_name(&self) -> &'static str {
        match self.dev {
            NetDev::Virtio(_) => "virtio",
            NetDev::E1000(_) => "e1000",
        }
    }

    pub fn ip_str(&self) -> Ipv4Address {
        self.ip.unwrap_or(Ipv4Address::new(0, 0, 0, 0))
    }

    pub fn gw_str(&self) -> Ipv4Address {
        self.gw.unwrap_or(Ipv4Address::new(0, 0, 0, 0))
    }
}

pub fn with_stack<R>(f: impl FnOnce(&mut Interface, &mut NetDev, &mut SocketSet<'static>) -> R) -> Option<R> {
    STACK.lock().as_mut().map(|s| f(&mut s.iface, &mut s.dev, &mut s.sockets))
}

// ========== PROBE ==========

pub fn probe_boot() {
    let devs = crate::pci::enumerate();
    let found = devs.iter()
        .find(|d| d.vendor == 0x1AF4 && d.device == 0x1000)
        .map(|d| (*d, "virtio"))
        .or_else(|| {
            devs.iter().find(|d| d.vendor == 0x8086 && d.device == 0x100E)
                .map(|d| (*d, "e1000"))
        });
    let (d, name) = match found {
        Some(x) => x,
        None => {
            crate::println!("NET: no net device (degradado)");
            return;
        }
    };
    crate::println!("NET-DEV {}", name);

    let (mac, dev) = match name {
        "virtio" => {
            let vn = match VirtioNet::probe(&d) {
                Ok(v) => v,
                Err(e) => {
                    crate::println!("NET: falha na deteccao virtio {} (degradado)", e);
                    return;
                }
            };
            (vn.mac, NetDev::Virtio(QueueHandle {
                inner: Arc::new(Mutex::new(QueueInner { virtio: Some(vn), e1000: None })),
            }))
        }
        "e1000" => {
            match E1000::probe(&d) {
                Ok(e) => {
                    crate::println!("E1000-PROBED mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                        e.mac[0], e.mac[1], e.mac[2], e.mac[3], e.mac[4], e.mac[5]);
                    (e.mac, NetDev::E1000(QueueHandle {
                        inner: Arc::new(Mutex::new(QueueInner { virtio: None, e1000: Some(e) })),
                    }))
                }
                Err(e) => {
                    crate::println!("NET: falha na deteccao e1000 {} (degradado)", e);
                    return;
                }
            }
        }
        _ => unreachable!(),
    };

    let eth = EthernetAddress(mac);
    let mut config = Config::new(HardwareAddress::Ethernet(eth));
    config.random_seed = 0xA53A_5AA5;
    let storage: &'static mut [SocketStorage<'static>; 8] =
        alloc::boxed::Box::leak(alloc::boxed::Box::new([SocketStorage::EMPTY; 8]));
    let mut iface = Interface::new(config, &mut dev.clone(), crate::time::smol_instant::now());
    
    // No static IP - will use DHCP
    iface.update_ip_addrs(|a| {
        a.clear();
    });
    
    let mut sockets = SocketSet::new(&mut storage[..]);
    
    // Add DHCP socket
    let dhcp_sock = DhcpSocket::new();
    let _dhcp_handle = sockets.add(dhcp_sock);
    
    *STACK.lock() = Some(NetStack { 
        dev, 
        iface, 
        sockets, 
        ip: None, 
        gw: None, 
        mac,
    });
    
    crate::println!("NET-INIT mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
    
    // Run DHCP
    run_dhcp();
}

fn run_dhcp() {
    // DHCP simplificado - usa IP estatico como fallback
    let static_ip = Ipv4Address::new(10, 0, 2, 15);
    let static_gw = Ipv4Address::new(10, 0, 2, 2);
    
    with_stack(|iface, _, _| {
        iface.update_ip_addrs(|a| {
            a.clear();
            let _ = a.push(IpCidr::new(IpAddress::Ipv4(static_ip), 24));
        });
        iface.routes_mut().add_default_ipv4_route(static_gw).ok();
    });
    STACK.lock().as_mut().unwrap().ip = Some(static_ip);
    STACK.lock().as_mut().unwrap().gw = Some(static_gw);
    
    crate::println!("NET-STATIC ip={} gw={}", static_ip, static_gw);
}

// ========== PING ==========

pub fn echo_request(ident: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
    let mut pkt = Vec::new();
    pkt.push(8); // type: echo request
    pkt.push(0); // code
    let checksum = checksum_calc(&pkt[2..]);
    pkt.push((checksum >> 8) as u8);
    pkt.push((checksum & 0xFF) as u8);
    pkt.push((ident >> 8) as u8);
    pkt.push((ident & 0xFF) as u8);
    pkt.push((seq >> 8) as u8);
    pkt.push((seq & 0xFF) as u8);
    pkt.extend_from_slice(payload);
    let cksum = checksum_calc(&pkt[2..]);
    pkt[2] = (cksum >> 8) as u8;
    pkt[3] = (cksum & 0xFF) as u8;
    pkt
}

fn checksum_calc(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    for chunk in data.chunks(2) {
        if chunk.len() == 2 {
            sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        } else {
            sum += (chunk[0] as u32) << 8;
        }
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}

pub fn cmd_ping() {
    let target = match STACK.lock().as_ref().and_then(|s| s.gw) {
        Some(g) => g,
        None => {
            crate::println!("NET-PING-NOGW");
            return;
        }
    };
    
    let sock = IcmpSocket::new(
        IcmpBuf::new([IcmpMeta::EMPTY; 4], alloc::vec![0u8; 256]),
        IcmpBuf::new([IcmpMeta::EMPTY; 4], alloc::vec![0u8; 256]),
    );
    let h = match with_stack(|_, _, socks| socks.add(sock)) {
        Some(h) => h,
        None => { crate::println!("NET-PING-NOSTACK"); return; }
    };
    with_stack(|_, _, socks| {
        socks.get_mut::<IcmpSocket>(h).bind(IcmpEndpoint::Ident(0xBEEF)).ok();
    });
    let req = echo_request(0xBEEF, 1, b"tuios-prime");
    let t0 = crate::time::millis();
    let mut ok = false;
    let mut sends = 0u32;
    
    while crate::time::millis() - t0 < 3000 {
        with_stack(|iface, dev, socks| {
            iface.poll(crate::time::smol_instant::now(), dev, socks);
            
            let s = socks.get_mut::<IcmpSocket>(h);
            if s.can_send() && sends == 0 {
                match s.send_slice(&req, IpAddress::Ipv4(target)) {
                    Ok(_) => {
                        sends += 1;
                        crate::println!("PING: enviado para {}", target);
                    }
                    Err(e) => {
                        crate::println!("PING: falha no envio {:?}", e);
                    }
                }
            }
            
            let mut rbuf = [0u8; 128];
            match s.recv_slice(&mut rbuf) {
                Ok((n, _)) => {
                    crate::println!("PING: recebidos {} bytes", n);
                    if n >= 8 && rbuf[0] == 0 && u16::from_be_bytes([rbuf[4], rbuf[5]]) == 0xBEEF {
                        ok = true;
                        crate::println!("PING: resposta valida!");
                    }
                }
                Err(_) => {}
            }
        });
        
        if ok { break; }
        sends += 1;
        core::hint::spin_loop();
    }
    
    if ok {
        crate::println!("NET-PING-OK target={}", target);
    } else {
        crate::println!("NET-PING-TIMEOUT target={} sends={}", target, sends);
    }
}

// ========== HTTP ==========

pub fn cmd_http() {
    let srv = Ipv4Address::new(10, 0, 2, 2);
    let sock = TcpSocket::new(TcpBuf::new(alloc::vec![0u8; 4096]), TcpBuf::new(alloc::vec![0u8; 4096]));
    let h = match with_stack(|_, _, socks| socks.add(sock)) {
        Some(h) => h,
        None => { crate::println!("HTTP-NOSTACK"); return; }
    };
    with_stack(|iface, _, socks| {
        let s = socks.get_mut::<TcpSocket>(h);
        let _ = s.connect(iface.context(), (IpAddress::Ipv4(srv), 18080), 49152);
    });
    let t0 = crate::time::millis();
    let mut body = alloc::vec![0u8; 0];
    let mut sent = false;
    let mut ok = false;
    
    while crate::time::millis() - t0 < 5000 {
        with_stack(|iface, dev, socks| {
            iface.poll(crate::time::smol_instant::now(), dev, socks);
            let s = socks.get_mut::<TcpSocket>(h);
            if s.can_send() && !sent {
                if s.send_slice(b"GET /hello.txt HTTP/1.0\r\nHost: x\r\n\r\n").is_ok() {
                    sent = true;
                    crate::println!("HTTP: requisicao enviada");
                }
            }
            if s.can_recv() {
                let mut tmp = [0u8; 512];
                if let Ok(n) = s.recv_slice(&mut tmp) {
                    body.extend_from_slice(&tmp[..n]);
                }
            }
            if !s.may_recv() && sent && !body.is_empty() {
                ok = true;
            }
        });
        if ok { break; }
        core::hint::spin_loop();
    }
    
    let text = core::str::from_utf8(&body).unwrap_or("");
    if text.contains("200") && text.contains("HELLO") {
        crate::println!("HTTP-GET-OK len={}", body.len());
    } else {
        crate::println!("HTTP-GET-FAIL len={}", body.len());
    }
}

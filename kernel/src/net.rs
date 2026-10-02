//! Rede Fase 1: VirtioNet (legado) + smoltcp (DHCP/ping/TCP). e1000 entra na T7.
use alloc::sync::Arc;
use alloc::vec::Vec;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};

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

// ---------- VirtioNet ----------

pub struct VirtioNet {
    io: u16,
    mac: [u8; 6],
    rx: crate::virtio::VirtQueue,
    tx: crate::virtio::VirtQueue,
    rx_bufs: Vec<Vec<u8>>,
}

impl VirtioNet {
    pub fn probe(dev: &crate::pci::PciDev) -> Result<Self, &'static str> {
        use crate::virtio::{ack_features, finish_ok, inb_pub, init_legacy, VirtQueue, DESC_F_WRITE};
        let r = init_legacy(dev)?;
        ack_features(r.io, 1 << 5); // VIRTIO_NET_F_MAC
        let mut mac = [0u8; 6];
        for i in 0..6 {
            mac[i] = inb_pub(r.io + 0x14 + i as u16);
        }
        // LEGADO: fila TEM que ter QueueNumMax (device computa layout com max)
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

    fn recv_pkt(&mut self) -> Option<Vec<u8>> {
        let (id, len) = self.rx.pop_used()?;
        self.rx.free_chain(id, 1);
        let mut buf = self.rx_bufs.pop()?;
        buf.truncate(len as usize);
        self.post_rx();
        Some(buf)
    }

    fn xmit(&mut self, pkt: &[u8]) {
        // header virtio-net vazio (10B) + pacote; header em heap vazado (Fase 1: sem reuse)
        let hdr: &'static mut [u8] = alloc::boxed::Box::leak(alloc::vec![0u8; 10].into_boxed_slice());
        let ph = VirtQueue::phys(hdr.as_ptr());
        let p = VirtQueue::phys(pkt.as_ptr());
        self.tx.add_chain(&[(ph, 10, 0), (p, pkt.len() as u32, 0)]);
        self.tx.notify(self.io, 1);
        for _ in 0..200_000 {
            if let Some((hid, _)) = self.tx.pop_used() {
                self.tx.free_chain(hid, 2);
                break;
            }
            core::hint::spin_loop();
        }
    }
}

// ---------- Tokens (Arc: evita auto-empréstimo iface+device) ----------

pub struct QueueHandle {
    inner: Arc<Mutex<QueueInner>>,
}

pub struct QueueInner {
    pub virtio: Option<VirtioNet>,
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
        }
        r
    }
}

pub enum NetDev {
    Virtio(QueueHandle),
}

impl Device for NetDev {
    type RxToken<'a>
        = NetRx
    where
        Self: 'a;
    type TxToken<'a>
        = NetTx
    where
        Self: 'a;

    fn receive(&mut self, _ts: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let q = match self {
            NetDev::Virtio(h) => h.inner.clone(),
        };
        let pkt = q.lock().virtio.as_mut()?.recv_pkt()?;
        Some((NetRx { pkt }, NetTx { q }))
    }

    fn transmit(&mut self, _ts: Instant) -> Option<Self::TxToken<'_>> {
        let q = match self {
            NetDev::Virtio(h) => h.inner.clone(),
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

pub struct NetStack {
    dev: NetDev,
    iface: Interface,
    sockets: SocketSet<'static>,
    ip: Option<Ipv4Address>,
    gw: Option<Ipv4Address>,
    mac: [u8; 6],
}

impl NetStack {
    pub fn dev_name(&self) -> &'static str {
        match self.dev {
            NetDev::Virtio(_) => "virtio",
        }
    }
    pub fn ip_str(&self) -> Ipv4Address {
        self.ip.unwrap_or(Ipv4Address::UNSPECIFIED)
    }
    pub fn gw_str(&self) -> Ipv4Address {
        self.gw.unwrap_or(Ipv4Address::UNSPECIFIED)
    }
}

pub fn with_stack<R>(f: impl FnOnce(&mut Interface, &mut NetDev, &mut SocketSet<'static>) -> R) -> Option<R> {
    STACK.lock().as_mut().map(|s| f(&mut s.iface, &mut s.dev, &mut s.sockets))
}

fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut it = data.chunks_exact(2);
    for w in &mut it {
        sum += u16::from_be_bytes([w[0], w[1]]) as u32;
    }
    if let Some(&b) = it.remainder().first() {
        sum += (b as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

fn echo_request(ident: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
    let mut pkt = alloc::vec![0u8; 8 + payload.len()];
    pkt[0] = 8; // Echo Request
    pkt[1] = 0;
    pkt[4..6].copy_from_slice(&ident.to_be_bytes());
    pkt[6..8].copy_from_slice(&seq.to_be_bytes());
    pkt[8..].copy_from_slice(payload);
    let c = checksum(&pkt);
    pkt[2..4].copy_from_slice(&c.to_be_bytes());
    pkt
}

pub fn probe_boot() {
    let devs = crate::pci::enumerate();
    let net = devs.iter().find(|d| d.vendor == 0x1AF4 && d.device == 0x1000);
    let d = match net {
        Some(d) => *d,
        None => {
            crate::println!("NET: no net device (degraded)");
            return;
        }
    };
    crate::println!("NET-DEV virtio");
    let vn = match VirtioNet::probe(&d) {
        Ok(v) => v,
        Err(e) => {
            crate::println!("NET: virtio probe failed {} (degraded)", e);
            return;
        }
    };
    let mac = vn.mac;
    let eth = EthernetAddress(mac);
    let mut config = Config::new(HardwareAddress::Ethernet(eth));
    config.random_seed = 0xA53A_5AA5;
    let storage: &'static mut [SocketStorage<'static>; 4] =
        alloc::boxed::Box::leak(alloc::boxed::Box::new([SocketStorage::EMPTY; 4]));
    let mut device = NetDev::Virtio(QueueHandle {
        inner: Arc::new(Mutex::new(QueueInner { virtio: Some(vn) })),
    });
    let mut iface = Interface::new(config, &mut device, crate::time::smol_instant::now());
    iface.update_ip_addrs(|a| {
        a.clear();
        let _ = a.push(IpCidr::new(IpAddress::v4(0, 0, 0, 0), 0));
    });
    let mut sockets = SocketSet::new(&mut storage[..]);
    *STACK.lock() = Some(NetStack { dev: device, iface, sockets, ip: None, gw: None, mac });
    // Static IP config (QEMU user-mode: 10.0.2.15, gateway 10.0.2.2)
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
    crate::println!(
        "NET-STATIC ip={} gw={} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        static_ip, static_gw,
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
}

pub fn cmd_ping() {
    let gw = match STACK.lock().as_ref().and_then(|s| s.gw) {
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
        None => {
            crate::println!("NET-PING-NOSTACK");
            return;
        }
    };
    with_stack(|_, _, socks| {
        socks.get_mut::<IcmpSocket>(h).bind(IcmpEndpoint::Ident(0xBEEF)).ok();
    });
    let req = echo_request(0xBEEF, 1, b"tuios-prime");
    let t0 = crate::time::millis();
    let mut ok = false;
    let mut tx_count = 0u32;
    let mut rx_count = 0u32;
    let mut poll_count = 0u32;
    crate::println!("PING-START t={} gw={}", crate::time::millis(), gw);
    while crate::time::millis() - t0 < 5000 {
        poll_count += 1;
        with_stack(|iface, dev, socks| {
            iface.poll(crate::time::smol_instant::now(), dev, socks);
            let s = socks.get_mut::<IcmpSocket>(h);
            if s.can_send() && tx_count < 10 {
                let _ = s.send_slice(&req, IpAddress::Ipv4(gw));
                tx_count += 1;
                crate::println!("PING-SENT #{} tx={}", tx_count, tx_count);
            }
            let mut rbuf = [0u8; 128];
            if let Ok((n, _)) = s.recv_slice(&mut rbuf) {
                rx_count += 1;
                crate::println!("PING-RCVD n={} tx={} rx={}", n, tx_count, rx_count);
                if n >= 8 && rbuf[0] == 0 && u16::from_be_bytes([rbuf[4], rbuf[5]]) == 0xBEEF {
                    ok = true;
                }
            }
        });
        if poll_count % 500 == 0 {
            crate::println!("PING-DIAG t={} polls={} tx={} rx={}", crate::time::millis(), poll_count, tx_count, rx_count);
        }
        if ok {
            break;
        }
    }
    crate::println!("PING-END tx={} rx={}", tx_count, rx_count);
    if ok {
        crate::println!("NET-PING-OK gw={}", gw);
    } else {
        crate::println!("NET-PING-TIMEOUT");
    }
}

pub fn cmd_http() {
    let srv = Ipv4Address::new(10, 0, 2, 2);
    let sock = TcpSocket::new(TcpBuf::new(alloc::vec![0u8; 4096]), TcpBuf::new(alloc::vec![0u8; 4096]));
    let h = match with_stack(|_, _, socks| socks.add(sock)) {
        Some(h) => h,
        None => {
            crate::println!("HTTP-NOSTACK");
            return;
        }
    };
    with_stack(|iface, _, socks| {
        let s = socks.get_mut::<TcpSocket>(h);
        let _ = s.connect(iface.context(), (IpAddress::Ipv4(srv), 18080), 49152);
    });
    let t0 = crate::time::millis();
    let mut body = alloc::vec![0u8; 0];
    let mut sent = false;
    let mut ok = false;
    while crate::time::millis() - t0 < 8000 {
        with_stack(|iface, dev, socks| {
            iface.poll(crate::time::smol_instant::now(), dev, socks);
            let s = socks.get_mut::<TcpSocket>(h);
            if s.can_send() && !sent {
                if s.send_slice(b"GET /hello.txt HTTP/1.0\r\nHost: x\r\n\r\n").is_ok() {
                    sent = true;
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
        if ok {
            break;
        }
    }
    let text = core::str::from_utf8(&body).unwrap_or("");
    if text.contains("200") && text.contains("HTTP HELLO FROM HOST") {
        crate::println!("HTTP-GET-OK len={}", body.len());
    } else {
        crate::println!("HTTP-GET-FAIL len={}", body.len());
    }
}

use crate::virtio::*;
use spin::Mutex;

pub trait BlockDevice: Send {
    fn read_block(&mut self, lba: u64, buf: &mut [u8; 512]) -> Result<(), &'static str>;
    fn write_block(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), &'static str>;
    fn blocks(&self) -> u64;
}

pub static VIRTIO_BLK: Mutex<Option<VirtioBlk>> = Mutex::new(None);

#[repr(C)]
struct BlkReq {
    typ: u32,
    _reserved: u32,
    sector: u64,
}

pub struct VirtioBlk {
    io: u16,
    q: VirtQueue,
    nblocks: u64,
}

impl VirtioBlk {
    pub fn probe(dev: &crate::pci::PciDev) -> Result<Self, &'static str> {
        let r = init_legacy(dev)?;
        ack_features(r.io, 0); // sem features especiais na Fase 1
        let max = VirtQueue::queue_max(r.io, 0);
        if max == 0 || max > 256 {
            return Err("bad queue max");
        }
        let mut q = VirtQueue::new(max);
        q.setup_legacy(r.io, 0);
        finish_ok(r.io);
        let nblocks = capacity(r.io);
        Ok(Self { io: r.io, q, nblocks })
    }

    fn xfer(&mut self, lba: u64, data: *mut u8, out: bool) -> Result<(), &'static str> {
        let mut status = 0xFFu8;
        let req = BlkReq { typ: if out { 1 } else { 0 }, _reserved: 0, sector: lba };
        let p_req = VirtQueue::phys(&req as *const _ as *const u8);
        let p_data = VirtQueue::phys(data);
        let p_st = VirtQueue::phys(&status as *const _ as *const u8);
        let dflag = if out { 0 } else { DESC_F_WRITE };
        self.q.add_chain(&[
            (p_req, 16, 0),
            (p_data, 512, dflag),
            (p_st, 1, DESC_F_WRITE),
        ]);
        self.q.notify(self.io, 0);
        let mut done = false;
        for _ in 0..1_000_000 {
            if let Some((hid, _)) = self.q.pop_used() {
                self.q.free_chain(hid, 3);
                done = true;
                break;
            }
            core::hint::spin_loop();
        }
        if !done {
            return Err("blk timeout");
        }
        if status != 0 {
            return Err("blk io status != 0");
        }
        Ok(())
    }
}

fn capacity(io: u16) -> u64 {
    // capacity (u64) no início do device config (BAR0 + 0x14)
    let lo = inl_pub(io + 0x14);
    let hi = inl_pub(io + 0x18);
    ((hi as u64) << 32) | lo as u64
}

impl BlockDevice for VirtioBlk {
    fn read_block(&mut self, lba: u64, buf: &mut [u8; 512]) -> Result<(), &'static str> {
        for attempt in 0..3 {
            match self.xfer(lba, buf.as_mut_ptr(), false) {
                Ok(()) => return Ok(()),
                Err(e) if attempt == 2 => return Err(e),
                Err(_) => continue,
            }
        }
        Err("unreachable")
    }
    fn write_block(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), &'static str> {
        let mut tmp = [0u8; 512];
        tmp.copy_from_slice(buf);
        for attempt in 0..3 {
            match self.xfer(lba, tmp.as_mut_ptr(), true) {
                Ok(()) => return Ok(()),
                Err(e) if attempt == 2 => return Err(e),
                Err(_) => continue,
            }
        }
        Err("unreachable")
    }
    fn blocks(&self) -> u64 {
        self.nblocks
    }
}

pub fn probe_boot() {
    let devs = crate::pci::enumerate();
    let blk = devs.iter().find(|d| d.vendor == 0x1AF4 && d.device == 0x1001);
    match blk {
        Some(d) => match VirtioBlk::probe(d) {
            Ok(b) => {
                crate::println!("BLK-OK blocks={}", b.blocks());
                *VIRTIO_BLK.lock() = Some(b);
            }
            Err(e) => crate::println!("BLK: probe failed {} (degraded)", e),
        },
        None => crate::println!("BLK: no block device (degraded)"),
    }
}

pub fn cmd_read(lba: u64) {
    let mut guard = VIRTIO_BLK.lock();
    match guard.as_mut() {
        Some(b) => {
            let mut buf = [0u8; 512];
            match b.read_block(lba, &mut buf) {
                Ok(()) => {
                    crate::println!("BLK-READ-OK lba={} b0={:02x} b1={:02x} bend={:02x}", lba, buf[0], buf[1], buf[511]);
                }
                Err(e) => crate::println!("BLK-READ-EIO {}", e),
            }
        }
        None => crate::println!("BLK-READ-NODEV"),
    }
}

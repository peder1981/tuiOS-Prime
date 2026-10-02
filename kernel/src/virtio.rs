//! Virtio LEGADO sobre PCI (sem parsing de capabilities; moderno fica Fase 3+).
use crate::pci::PciDev;

pub const REG_FEATURES: u16 = 0x00;
pub const REG_GUEST_FEATURES: u16 = 0x04;
pub const REG_QUEUE_PFN: u16 = 0x08;
pub const REG_QUEUE_NUM: u16 = 0x0C;
pub const REG_QUEUE_SEL: u16 = 0x0E;
pub const REG_QUEUE_NOTIFY: u16 = 0x10;
pub const REG_STATUS: u16 = 0x12;

pub const ST_ACK: u8 = 1;
pub const ST_DRIVER: u8 = 2;
pub const ST_DRIVER_OK: u8 = 4;

pub const DESC_F_NEXT: u16 = 1;
pub const DESC_F_WRITE: u16 = 2;

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct Desc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

pub struct LegacyRegs {
    pub io: u16,
}

fn outb(p: u16, v: u8) {
    unsafe { x86_64::instructions::port::Port::<u8>::new(p).write(v) };
}
fn outw(p: u16, v: u16) {
    unsafe { x86_64::instructions::port::Port::<u16>::new(p).write(v) };
}
fn outl(p: u16, v: u32) {
    unsafe { x86_64::instructions::port::Port::<u32>::new(p).write(v) };
}
fn inb(p: u16) -> u8 {
    unsafe { x86_64::instructions::port::Port::<u8>::new(p).read() }
}
fn inw(p: u16) -> u16 {
    unsafe { x86_64::instructions::port::Port::<u16>::new(p).read() }
}
fn inl(p: u16) -> u32 {
    unsafe { x86_64::instructions::port::Port::<u32>::new(p).read() }
}
pub fn inb_pub(p: u16) -> u8 {
    inb(p)
}
pub fn inl_pub(p: u16) -> u32 {
    inl(p)
}

pub fn init_legacy(dev: &PciDev) -> Result<LegacyRegs, &'static str> {
    use crate::pci::{bar, enable_bus_master};
    enable_bus_master(dev);
    let (addr, is_io) = bar(dev, 0);
    if !is_io {
        return Err("virtio bar0 not IO");
    }
    let io = addr as u16;
    outb(io + REG_STATUS, 0); // reset
    outb(io + REG_STATUS, ST_ACK | ST_DRIVER);
    Ok(LegacyRegs { io })
}

pub fn finish_ok(io: u16) {
    outb(io + REG_STATUS, ST_ACK | ST_DRIVER | ST_DRIVER_OK);
}

pub fn ack_features(io: u16, mask: u32) {
    let f = inl(io + REG_FEATURES) & mask;
    outl(io + REG_GUEST_FEATURES, f);
}

// Single-core Fase 1: anéis acessados só com interrupts desabilitados ou em
// seção crítica do driver; envio entre threads nunca acontece.
unsafe impl Send for VirtQueue {}

pub struct VirtQueue {
    pub size: u16,
    pub desc: *mut Desc, // 16-alinhado, vazado de propósito (vive p/ sempre)
    pub avail: *mut u16, // avail ring (flags@0 idx@1 ring@2..)
    pub used: *mut u8,   // used ring base
    pub free_head: u16,
    pub avail_idx: u16,
    pub last_used: u16,
}

impl Clone for VirtQueue {
    fn clone(&self) -> Self {
        // Nota: isso compartilha os ponteiros - uso apenas em contexto single-thread
        Self {
            size: self.size,
            desc: self.desc,
            avail: self.avail,
            used: self.used,
            free_head: self.free_head,
            avail_idx: self.avail_idx,
            last_used: self.last_used,
        }
    }
}

impl VirtQueue {
    pub fn new(size: u16) -> Self {
        // Layout LEGADO contíguo: desc[16*size] + avail[6+2*size] + used(4K-alinhado)[8+8*size].
        // O device deriva avail/used do PFN — buffers separados NUNCA seriam vistos.
        let desc_bytes = size as usize * 16;
        let avail_bytes = 6 + 2 * size as usize;
        let used_off = (desc_bytes + avail_bytes + 4095) & !4095;
        let total = used_off + 8 + 8 * size as usize;
        let raw = alloc::vec![0u8; total + 4096];
        let base = (raw.as_ptr() as usize + 4095) & !4095; // PFN exige página
        core::mem::forget(raw);
        let mut vq = Self {
            size,
            desc: base as *mut Desc,
            avail: (base + desc_bytes) as *mut u16,
            used: (base + used_off) as *mut u8,
            free_head: 0,
            avail_idx: 0,
            last_used: 0,
        };
        for i in 0..size {
            vq.d(i).next = (i + 1) % size;
        }
        vq
    }

    fn d(&mut self, i: u16) -> &mut Desc {
        unsafe { &mut *self.desc.add(i as usize) }
    }

    pub fn phys(p: *const u8) -> u64 {
        (p as u64).wrapping_sub(crate::heap::hhdm_offset() as u64)
    }

    /// Seleciona a fila e devolve QueueNumMax. LEGADO NÃO TEM size-select:
    /// o device sempre computa o layout com max — a fila TEM que ter esse tamanho.
    pub fn queue_max(io: u16, sel: u16) -> u16 {
        outw(io + REG_QUEUE_SEL, sel);
        inw(io + REG_QUEUE_NUM)
    }

    pub fn setup_legacy(&mut self, io: u16, sel: u16) {
        outw(io + REG_QUEUE_SEL, sel);
        let pfn = (Self::phys(self.desc as *const u8) >> 12) as u32;
        outl(io + REG_QUEUE_PFN, pfn);
    }

    pub fn add_chain(&mut self, parts: &[(u64, u32, u16)]) -> u16 {
        let head = self.free_head;
        let mut prev: Option<u16> = None;
        for (i, (addr, len, flags)) in parts.iter().enumerate() {
            let idx = if i == 0 { head } else { self.d(prev.unwrap()).next };
            let dd = self.d(idx);
            dd.addr = *addr;
            dd.len = *len;
            dd.flags = *flags;
            if let Some(p) = prev {
                let pd = self.d(p);
                pd.flags |= DESC_F_NEXT;
                pd.next = idx;
            }
            prev = Some(idx);
        }
        let last = prev.unwrap();
        let ln = self.d(last);
        ln.flags &= !DESC_F_NEXT;
        self.free_head = ln.next;
        // Volatile: o device (DMA) lê/escreve esses anéis concorrentemente;
        // acesso normal permite ao compilador hoistear leitura p/ fora do loop de poll.
        // ATENÇÃO offsets em BYTES (avail é *mut u16: .add() anda 2B!): flags@0 idx@2 ring@4
        unsafe {
            let ring = self.avail.byte_add(4) as *mut u16;
            core::ptr::write_volatile(
                ring.add((self.avail_idx % self.size) as usize),
                head,
            );
            self.avail_idx += 1;
            core::ptr::write_volatile(self.avail.byte_add(2) as *mut u16, self.avail_idx);
        }
        head
    }

    pub fn notify(&self, io: u16, qsel: u16) {
        outw(io + REG_QUEUE_NOTIFY, qsel);
    }

    /// Desenfileira UMA conclusão (puro: não toca a free list).
    pub fn pop_used(&mut self) -> Option<(u16, u32)> {
        unsafe {
            let used_idx = core::ptr::read_volatile(self.used.add(2) as *const u16);
            if self.last_used == used_idx {
                return None;
            }
            let elem = self.used.add(4).cast::<[u32; 2]>().add((self.last_used % self.size) as usize);
            let id = core::ptr::read_volatile(&(*elem)[0]);
            let len = core::ptr::read_volatile(&(*elem)[1]);
            self.last_used += 1;
            Some((id as u16, len))
        }
    }

    /// Devolve `n` descritores a partir de `head`. Cadeias sempre do mesmo tamanho
    /// por chamador (blk=3, net-tx=2, net-rx=1). Conclusão síncrona => head é o nosso.
    pub fn free_chain(&mut self, head: u16, n: usize) {
        let mut cur = head;
        for _ in 0..n {
            let nx = self.d(cur).next;
            self.d(cur).next = self.free_head;
            self.free_head = cur;
            cur = nx;
        }
    }
}

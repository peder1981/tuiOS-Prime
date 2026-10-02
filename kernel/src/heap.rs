use core::sync::atomic::{AtomicUsize, Ordering};
use linked_list_allocator::LockedHeap;

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

static HHDM_OFF: AtomicUsize = AtomicUsize::new(0);

pub fn hhdm_offset() -> usize {
    HHDM_OFF.load(Ordering::Relaxed)
}

/// Perfil de hardware detectado
pub enum HardwareProfile {
    Minimal,    // 2GB RAM, Atom/Celeron
    Standard,   // 4GB RAM, Core m/Pentium
    Performance, // 8GB+ RAM, Core i5/i7
}

impl HardwareProfile {
    /// Retorna tamanho recomendado de heap
    pub const fn heap_size(&self) -> usize {
        match self {
            HardwareProfile::Minimal => 8 * 1024 * 1024,     // 8MB
            HardwareProfile::Standard => 16 * 1024 * 1024,   // 16MB
            HardwareProfile::Performance => 32 * 1024 * 1024, // 32MB
        }
    }
}

// Padrao: Standard (16MB) - pode ser alterado via variavel de ambiente
pub const HEAP_BYTES: usize = HardwareProfile::Standard.heap_size();

pub fn init_heap() -> Result<usize, &'static str> {
    let (memmap, hhdm) = crate::limine_requests();
    let memmap = memmap.ok_or("no memmap")?;
    let hhdm = hhdm.ok_or("no hhdm")? as usize;
    crate::println!("HHDM-OFF={:#x}", hhdm);
    HHDM_OFF.store(hhdm, Ordering::Relaxed);
    for e in memmap.entries() {
        if e.type_ == limine::memmap::MEMMAP_USABLE && (e.length as usize) >= HEAP_BYTES {
            let base = e.base as usize + hhdm;
            unsafe { HEAP.lock().init(base as *mut u8, HEAP_BYTES) };
            return Ok(base);
        }
    }
    Err("no usable region >= 16MiB")
}

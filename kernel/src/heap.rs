use core::sync::atomic::{AtomicUsize, Ordering};
use linked_list_allocator::LockedHeap;

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

static HHDM_OFF: AtomicUsize = AtomicUsize::new(0);

pub fn hhdm_offset() -> usize {
    HHDM_OFF.load(Ordering::Relaxed)
}

pub const HEAP_BYTES: usize = 16 * 1024 * 1024;

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

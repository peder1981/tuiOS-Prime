#![no_std]
#![feature(abi_x86_interrupt)]
#![no_main]

extern crate alloc;

mod arch;
mod heap;
mod mem;
mod net;
mod blk;
mod fs;
mod pci;
mod virtio;
mod time;
mod serial;
mod shell;
mod ahci;

use core::panic::PanicInfo;
use limine::request::{HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;

// Bloco único repr(C): ordem garantida start < hhdm < memmap < base < end,
// que é o que o Limine 8.x exige ao escanear a imagem carregada.
#[repr(C)]
struct RequestBlock {
    start: RequestsStartMarker,
    hhdm: HhdmRequest,
    memmap: MemoryMapRequest,
    base: BaseRevision,
    end: RequestsEndMarker,
}

#[used]
#[unsafe(link_section = ".limine_requests")]
static REQUEST_BLOCK: RequestBlock = RequestBlock {
    start: RequestsStartMarker::new(),
    hhdm: HhdmRequest::new(),
    memmap: MemoryMapRequest::new(),
    base: BaseRevision::new(),
    end: RequestsEndMarker::new(),
};

pub fn limine_requests() -> (
    Option<&'static limine::response::MemoryMapResponse>,
    Option<u64>,
) {
    let memmap = REQUEST_BLOCK.memmap.get_response();
    let hhdm = REQUEST_BLOCK.hhdm.get_response().map(|r| r.offset());
    (memmap, hhdm)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial::init();
    println!("HELLO-TUIOS-KERNEL");
    match heap::init_heap() {
        Ok(_) => println!("HEAP-OK"),
        Err(e) => println!("HEAP-FAIL {}", e),
    }
    arch::init();
    let devs = pci::enumerate();
    println!("PCI-OK n={}", devs.len());
    for d in &devs {
        println!("pci {:02x}:{:02x}.{} {:04x}:{:04x} class={:02x}:{:02x}",
            d.bus, d.dev, d.func, d.vendor, d.device, d.class, d.subclass);
    }
    blk::probe_boot();
    fs::mount();
    net::probe_boot();
    // ahci::probe_boot(); // TODO: MMIO mapping required
    println!("DRIVERS: virtio-blk probed");
    shell::run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}

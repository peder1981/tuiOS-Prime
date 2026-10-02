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
mod gpu;
mod audio;

use core::panic::PanicInfo;
use limine::{request::{HhdmRequest, MemmapRequest}, RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;

// Bloco único repr(C): ordem garantida start < hhdm < memmap < base < end,
// que é o que o Limine 8.x exige ao escanear a imagem carregada.
#[repr(C)]
struct RequestBlock {
    start: RequestsStartMarker,
    hhdm: HhdmRequest,
    memmap: MemmapRequest,
    base: BaseRevision,
    end: RequestsEndMarker,
}

#[used]
#[unsafe(link_section = ".limine_requests")]
static REQUEST_BLOCK: RequestBlock = RequestBlock {
    start: RequestsStartMarker::new(),
    hhdm: HhdmRequest::new(),
    memmap: MemmapRequest::new(),
    base: BaseRevision::new(),
    end: RequestsEndMarker::new(),
};

pub fn limine_requests() -> (
    Option<&'static limine::request::MemmapResponse>,
    Option<u64>,
) {
    let memmap = REQUEST_BLOCK.memmap.response();
    let hhdm = REQUEST_BLOCK.hhdm.response().map(|r| r.offset);
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
    let (_memmap, hhdm) = limine_requests();
    let hhdm = hhdm.unwrap_or(0xffff800000000000) as u64;
    unsafe { arch::map_mmio_regions(hhdm) };
    gpu::init();
    audio::init();
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
    ahci::probe_boot();
    println!("DRIVERS: all probed");
    // Auto network diag (Fase 2)
    crate::println!("NET-AUTO-TEST");
    crate::net::cmd_ping();
    shell::run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}

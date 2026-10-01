#![no_std]
#![no_main]

use core::fmt::Write;
use limine::request::{RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;
use spinning_top::Spinlock;
use uart_16550::SerialPort;

// Single repr(C) block => field order guaranteed: start < base < end,
// which is what Limine 8.x requires when scanning the loaded image.
#[repr(C)]
struct RequestBlock {
    start: RequestsStartMarker,
    base: BaseRevision,
    end: RequestsEndMarker,
}

#[used]
#[unsafe(link_section = ".limine_requests")]
static REQUEST_BLOCK: RequestBlock = RequestBlock {
    start: RequestsStartMarker::new(),
    base: BaseRevision::new(),
    end: RequestsEndMarker::new(),
};

static SERIAL: Spinlock<SerialPort> = Spinlock::new(unsafe { SerialPort::new(0x3F8) });

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut port = SERIAL.lock();
    port.init();
    let _ = writeln!(port, "HELLO-TUIOS-KERNEL");
    let _ = writeln!(port, "DRIVERS: none (fase0 ram+serial only)");
    drop(port);
    halt();
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut port = SERIAL.lock();
    let _ = writeln!(port, "PANIC: {}", info);
    halt();
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

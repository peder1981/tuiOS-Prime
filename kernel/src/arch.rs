use core::sync::atomic::{AtomicU64, Ordering};
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::instructions::segmentation::{Segment, CS, DS};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC1: u8 = 0x20;
pub const PIC2: u8 = 0x28;
pub const TIMER_IRQ: u8 = 0;

pub static TICKS: AtomicU64 = AtomicU64::new(0);
static PICS: Mutex<ChainedPics> = unsafe { Mutex::new(ChainedPics::new(PIC1, PIC2)) };

extern "x86-interrupt" fn timer_tick(_f: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe { PICS.lock().notify_end_of_interrupt(TIMER_IRQ) };
}

extern "x86-interrupt" fn breakpoint(_f: InterruptStackFrame) {
    crate::println!("BREAKPOINT-OK");
}

fn pit_hz(divisor: u16) {
    let mut cmd: Port<u8> = Port::new(0x43);
    let mut ch0: Port<u8> = Port::new(0x40);
    unsafe {
        cmd.write(0x36);
        ch0.write((divisor & 0xFF) as u8);
        ch0.write((divisor >> 8) as u8);
    }
}

pub fn init() {
    // GDT/IDT vivem para sempre: Box::leak (sem lazy_static na Fase 1)
    let gdt: &'static mut GlobalDescriptorTable = alloc::boxed::Box::leak(
        alloc::boxed::Box::new(GlobalDescriptorTable::new()),
    );
    let code: SegmentSelector = gdt.append(Descriptor::kernel_code_segment());
    let data: SegmentSelector = gdt.append(Descriptor::kernel_data_segment());
    gdt.load();
    unsafe {
        CS::set_reg(code);
        DS::set_reg(data);
    }
    let idt: &'static mut InterruptDescriptorTable = alloc::boxed::Box::leak(
        alloc::boxed::Box::new(InterruptDescriptorTable::new()),
    );
    idt[32].set_handler_fn(timer_tick);
    idt.breakpoint.set_handler_fn(breakpoint);
    idt.load();
    unsafe { PICS.lock().initialize() };
    pit_hz(1193); // 1193182/1193 ~= 1 kHz
    x86_64::instructions::interrupts::enable();
    crate::println!("TIME-OK base={}", TICKS.load(Ordering::Relaxed));
}

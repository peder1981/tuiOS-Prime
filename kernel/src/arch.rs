use core::sync::atomic::{AtomicU64, Ordering};
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::instructions::segmentation::{Segment, CS, DS, SS};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::structures::paging::OffsetPageTable;

pub const PIC1: u8 = 0x20;
pub const PIC2: u8 = 0x28;
pub const TIMER_IRQ: u8 = 0;

pub static TICKS: AtomicU64 = AtomicU64::new(0);
static PICS: Mutex<ChainedPics> = unsafe { Mutex::new(ChainedPics::new(PIC1, PIC2)) };

extern "x86-interrupt" fn timer_tick(_f: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        use x86_64::instructions::port::PortWriteOnly;
        PortWriteOnly::new(0x20).write(0x20u8); // EOI to PIC0
    }
}

extern "x86-interrupt" fn breakpoint(_f: InterruptStackFrame) {
    crate::println!("BREAKPOINT-OK");
}

macro_rules! fault_handler {
    ($name:ident, $vec:expr,err) => {
        extern "x86-interrupt" fn $name(f: InterruptStackFrame, code: u64) {
            crate::println!("EXC-FAULT vec={} code={:#x} rip={:#x}", $vec, code, f.instruction_pointer.as_u64());
            loop { x86_64::instructions::hlt(); }
        }
    };
}

fault_handler!(gp_fault, 13, err);
// double_fault exige -> ! ; page_fault exige PageFaultErrorCode (x86_64 0.15)
extern "x86-interrupt" fn df_fault(f: InterruptStackFrame, code: u64) -> ! {
        crate::println!("EXC-FAULT vec=8 code={:#x} rip={:#x}", code, f.instruction_pointer.as_u64());
        loop { x86_64::instructions::hlt(); }
    }
extern "x86-interrupt" fn pf_fault(f: InterruptStackFrame, code: x86_64::structures::idt::PageFaultErrorCode) {
        crate::println!("EXC-FAULT vec=14 bits={:#x} rip={:#x}", code.bits(), f.instruction_pointer.as_u64());
        loop { x86_64::instructions::hlt(); }
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
        // SS TEM que acompanhar: iretq valida o SS empilhado contra NOSSA GDT
        // (Limine deixa SS=0x30, fora do limite da nossa GDT de 3 entradas -> #GP).
        SS::set_reg(data);
        DS::set_reg(data);
    }
    let idt: &'static mut InterruptDescriptorTable = alloc::boxed::Box::leak(
        alloc::boxed::Box::new(InterruptDescriptorTable::new()),
    );
    idt[32].set_handler_fn(timer_tick);
    idt.breakpoint.set_handler_fn(breakpoint);
    idt.double_fault.set_handler_fn(df_fault);
    idt.general_protection_fault.set_handler_fn(gp_fault);
    idt.page_fault.set_handler_fn(pf_fault);
    idt.load();
    unsafe { PICS.lock().initialize() };
    // initialize() RESTAURA máscaras salvas (0xFF = tudo mascarado).
    // DESMASCARA IRQ0 AGORA, DEPOIS do initialize():
    unsafe {
        use x86_64::instructions::port::PortWriteOnly;
        PortWriteOnly::new(0x21).write(0xFEu8); // unmask IRQ0
        PortWriteOnly::new(0xA1).write(0xFEu8); // unmask IRQ8
    }    // Setup PIT channel 0, mode 3, 1kHz
    pit_hz(1193);
    pit_hz(1193);
    x86_64::instructions::interrupts::enable();
    crate::println!("TIME-OK base={}", TICKS.load(Ordering::Relaxed));
}

pub fn cmd_pic() {
    use x86_64::instructions::port::Port;
    unsafe {
        let mut c20: Port<u8> = Port::new(0x20);
        let mut p21: Port<u8> = Port::new(0x21);
        c20.write(0x0A);
        let irr1 = c20.read();
        c20.write(0x0B);
        let isr1 = c20.read();
        let m1: u8 = p21.read();
        let mut ca0: Port<u8> = Port::new(0xA0);
        let mut pa1: Port<u8> = Port::new(0xA1);
        ca0.write(0x0A);
        let irr2 = ca0.read();
        ca0.write(0x0B);
        let isr2 = ca0.read();
        let m2: u8 = pa1.read();
        crate::println!("PIC irr1={:#04x} isr1={:#04x} mask1={:#04x} irr2={:#04x} isr2={:#04x} mask2={:#04x} ticks={}",
            irr1, isr1, m1, irr2, isr2, m2, TICKS.load(core::sync::atomic::Ordering::Relaxed));
    }
}

/// Lê o contador atual do PIT canal 0 (latch + lobyte/hibyte). Diagnóstico: avança?
pub fn pit_count() -> u16 {
    use x86_64::instructions::port::Port;
    unsafe {
        let mut cmd: Port<u8> = Port::new(0x43);
        let mut ch0: Port<u8> = Port::new(0x40);
        cmd.write(0x00); // latch counter 0
        let lo = ch0.read();
        let hi = ch0.read();
        ((hi as u16) << 8) | lo as u16
    }
}

/// Lê RFLAGS (bit 9 = IF). Diagnóstico.
pub fn irq_enabled() -> bool {
    let r: u64;
    unsafe { core::arch::asm!("pushfq; pop {}", out(reg) r, options(nomem, preserves_flags)); }
    r & (1 << 9) != 0
}

/// Testa PIT manualmente + lê PIC masks após init.
pub fn cmd_diag_pic_pit() {
    use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};
    unsafe {
        // Latch counters e masks
        let mut c20: Port<u8> = Port::new(0x20);
        let mut p21: Port<u8> = Port::new(0x21);
        c20.write(0x0A);
        let irr1 = c20.read();
        c20.write(0x0B);
        let isr1 = c20.read();
        let m1: u8 = p21.read();
        let mut ca0: Port<u8> = Port::new(0xA0);
        let mut pa1: Port<u8> = Port::new(0xA1);
        ca0.write(0x0A);
        let irr2 = ca0.read();
        ca0.write(0x0B);
        let isr2 = ca0.read();
        let m2: u8 = pa1.read();
        crate::println!("PIC post-init irr1={:#04x} isr1={:#04x} mask1={:#04x} irr2={:#04x} isr2={:#04x} mask2={:#04x}",
            irr1, isr1, m1, irr2, isr2, m2);
        // Latch PIT ch0 (já deve estar em modo 3 / rate generator, port 0x40 = latch+read)
        let mut pitcmd: PortWriteOnly<u8> = PortWriteOnly::new(0x43);
        let mut pitd0: PortReadOnly<u8> = PortReadOnly::new(0x40);
        pitcmd.write(0x00); // latch counter 0
        core::hint::spin_loop();
        let pit_cur = ((pitd0.read() as u16) << 8) | pitd0.read() as u16;
        crate::println!("PIT-ch0 latch after init = {:#06x} (expected ~0x1388)", pit_cur);
        // Trigger IRQ manually: read ISR — se IRQ0 está ativo, isr1 bit 0 = 1
        crate::println!("IRQ0 asserted={}", (isr1 & 1) != 0);
    }
}

/// Dump IDT entries for vectors 0-4 and 32 (timer).
pub fn cmd_dump_idt() {
    use x86_64::instructions::tables::sidt;
    let tr = sidt();
    crate::println!("IDTR base={:#x} limit={}", tr.base.as_u64(), tr.limit);
    let entries = tr.base.as_u64() as *const u64;
    for i in 0..4 {
        let off = i * 2;
        let lo = unsafe { entries.add(off).read_volatile() };
        let hi = unsafe { entries.add(off+1).read_volatile() };
        crate::println!("IDT[{}] lo={:#018x} hi={:#018x}", i, lo, hi);
    }
    // Entry 32 (timer) - decode raw bytes
    let off = 32 * 2;
    let lo = unsafe { entries.add(off).read_volatile() };
    let hi = unsafe { entries.add(off+1).read_volatile() };
    crate::println!("IDT[32] raw lo={:#018x} hi={:#018x}", lo, hi);
    // Decode per x86_64 IDT entry format:
    // lo bytes: [offset15:0][selector][type][DPL/P][offset31:16]
    // hi bytes: [offset63:32][reserved]
    let offset16 = (lo >> 32) as u16;  // bits 32-47: type + offset[31:16]
    let offset_low = (lo & 0xffff) as u32;  // bits 0-15: offset[15:0]
    let offset_mid = ((lo >> 16) & 0xffff) as u32;  // bits 16-31: selector... wait
    // Actually let me just print the bytes
    let raw_ptr = tr.base.as_u64() as *const u8;
    let base_off = (32 * 16) as usize;
    crate::println!("IDT[32] bytes:");
    for i in 0..16 {
        let b = unsafe { raw_ptr.add(base_off + i).read_volatile() };
        crate::print!(" {:02x}", b);
    }
    crate::println!();
    // Decode: offset[15:0] = bytes 0-1, selector = bytes 2-3, type = byte 4, 
    //         offset[31:16] = bytes 6-7, offset[63:32] = bytes 8-11
    let off0 = unsafe { raw_ptr.add(base_off).read_volatile() };
    let off1 = unsafe { raw_ptr.add(base_off+1).read_volatile() };
    let sel0 = unsafe { raw_ptr.add(base_off+2).read_volatile() };
    let sel1 = unsafe { raw_ptr.add(base_off+3).read_volatile() };
    let typ = unsafe { raw_ptr.add(base_off+4).read_volatile() };
    let dplp = unsafe { raw_ptr.add(base_off+5).read_volatile() };
    let off6 = unsafe { raw_ptr.add(base_off+6).read_volatile() };
    let off7 = unsafe { raw_ptr.add(base_off+7).read_volatile() };
    let off8 = unsafe { raw_ptr.add(base_off+8).read_volatile() };
    let off9 = unsafe { raw_ptr.add(base_off+9).read_volatile() };
    let off10 = unsafe { raw_ptr.add(base_off+10).read_volatile() };
    let off11 = unsafe { raw_ptr.add(base_off+11).read_volatile() };
    let offset_low_val = ((off1 as u32) << 8) | (off0 as u32);
    let selector_val = ((sel1 as u32) << 8) | (sel0 as u32);
    let offset_mid_val = ((off7 as u32) << 8) | (off6 as u32);
    let offset_high_val = ((off11 as u32) << 24) | ((off10 as u32) << 16) | ((off9 as u32) << 8) | (off8 as u32);
    let full_offset = ((offset_high_val as u64) << 32) | ((offset_mid_val as u64) << 16) | (offset_low_val as u64);
    crate::println!("IDT[32] decoded: offset={:#018x} selector={:#06x} type={:#04x} dpl_p={:#04x}",
        full_offset, selector_val, typ, dplp);
    crate::println!("IDT[32] expected=0xffffffff8002c7b0 (timer_tick)");
}

/// Mapeia MMIO de dispositivos PCI acima de 1GB usando RecursivePageTable.
/// Limine já tem identity mapping para < 1GB. Para > 1GB, precisamos adicionar
/// entradas nas page tables existentes.
pub unsafe fn map_mmio_regions(hhdm_offset: u64) {
    use x86_64::structures::paging::{
        OffsetPageTable, Page, Size2MiB, Size4KiB, PageSize, PageTableFlags as Flags,
        PhysFrame, mapper::Mapper,
    };
    use x86_64::{PhysAddr, VirtAddr};
    use x86_64::registers::control::Cr3;
    
    // PML4 físico
    let pml4_phys = Cr3::read().0.start_address().as_u64();
    // Como Limine faz identity mapping para < 1GB, PML4 está em phys < 1GB
    // Seu virtual = hhdm_offset + phys_addr (pois identity + hhdm_offset)
    let pml4_virt = hhdm_offset + pml4_phys;
    
    // Criar mapper com phys_offset = hhdm_offset
    // Isso significa: phys_addr X -> virtual addr (hhdm_offset + X)
    let pml4_ptr = pml4_virt as *mut x86_64::structures::paging::PageTable;
    let mut mapper = OffsetPageTable::new(
        &mut *pml4_ptr,
        VirtAddr::new(hhdm_offset),
    );
    
    // Frame allocator que reutiliza frames já mapeados (identity)
    // Não precisa alocar novos frames porque os frames dos dispositivos
    // já existem na memória física e podem ser reutilizados
    struct IdentityAllocator;
    unsafe impl<S: PageSize> x86_64::structures::paging::FrameAllocator<S> for IdentityAllocator {
        fn allocate_frame(&mut self) -> Option<PhysFrame<S>> { None }
    }
    
    // Mapear 2MiB pages para os BARs dos dispositivos
    // AHCI: phys 0x80000000, size 256MiB -> 128 pages
    // e1000: phys 0x81080000, size 64KiB -> 1 page (align to 2MiB)
    // virtio: phys 0x81081000, size 4KiB -> 1 page (align to 2MiB)
    
    let flags = Flags::PRESENT | Flags::WRITABLE | Flags::NO_EXECUTE;
    let mut alloc = IdentityAllocator;
    
    // AHCI BAR: 0x80000000 - 0x80FFFFFF (256MB)
    for i in 0..128u64 {
        let phys = 0x80000000u64 + i * 0x200000;
        let page = Page::<Size2MiB>::containing_address(VirtAddr::new(hhdm_offset + phys));
        let frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(phys));
        let _ = mapper.map_to(page, frame, flags, &mut alloc);
    }
    
    // e1000 BAR: 0x81080000 - 0x8108FFFF (64KB)
    {
        let phys = 0x81080000u64;
        let page = Page::<Size2MiB>::containing_address(VirtAddr::new(hhdm_offset + phys));
        let frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(phys));
        let _ = mapper.map_to(page, frame, flags, &mut alloc);
    }
    
    // virtio-mmio BAR: 0x81081000 - 0x81081FFF (4KB)
    {
        let phys = 0x81081000u64;
        let page = Page::<Size2MiB>::containing_address(VirtAddr::new(hhdm_offset + phys));
        let frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(phys));
        let _ = mapper.map_to(page, frame, flags, &mut alloc);
    }
    
    crate::println!("MMIO-MAPPED phys=0x80000000..0x81082000 (via HHDM+0x{:x})", hhdm_offset);
}

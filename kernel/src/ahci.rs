//! AHCI (SATA) controller driver — leitura sem reset do HBA.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::{read_volatile as inl, write_volatile as outl};

// Per-port registers (offset 0x100 + port*0x80)
const PXCLB: u32 = 0x00;
const PXCLBU: u32 = 0x04;
const PXFB: u32 = 0x08;
const PXFBU: u32 = 0x0C;
const PXIS: u32 = 0x10;
const PXCMD: u32 = 0x18;
const PXSSTS: u32 = 0x28;
const PXCI: u32 = 0x38;

const CMD_ST: u32 = 1 << 0;
const CMD_FRE: u32 = 1 << 4;
const CMD_CR: u32 = 1 << 15;

#[repr(C, align(4096))]
struct CmdHeader {
    flags: u16,
    prdtl: u16,
    prdbc: u32,
    ctba: u32,
    ctbau: u32,
    reserved: [u32; 4],
}

fn abar_r(base: *mut u8, off: u32) -> u32 {
    unsafe { inl(base.add(off as usize) as *const u32) }
}
fn abar_w(base: *mut u8, off: u32, v: u32) {
    unsafe { outl(base.add(off as usize) as *mut u32, v) }
}

fn phys(p: *const u8) -> u64 {
    (p as u64).wrapping_sub(crate::heap::hhdm_offset() as u64)
}

pub fn probe_boot() {
    let ctrl = crate::pci::enumerate().into_iter()
        .find(|d| d.class == 0x01 && d.subclass == 0x06);
    let c = match ctrl {
        Some(c) => c,
        None => {
            crate::println!("AHCI: no ahci controller (degraded)");
            return;
        }
    };
    crate::pci::enable_bus_master(&c);
    let (addr, is_io) = crate::pci::bar(&c, 5);
    if is_io {
        crate::println!("AHCI: abar not MMIO (degraded)");
        return;
    }
    let hhdm = crate::heap::hhdm_offset() as u64;
        let abar = (addr + hhdm) as *mut u8;

    // Find port with device
    let mut found_port: Option<u32> = None;
    for p in 0..6u32 {
        let po = 0x100 + p * 0x80;
        let ssts = abar_r(abar, po + PXSSTS);
        let det = (ssts & 0xF) as u8;
        let sig = abar_r(abar, po + 0x24);
        if det == 3 && sig == 0x101 {
            found_port = Some(po);
            break;
        }
    }
    let po = match found_port {
        Some(po) => po,
        None => {
            crate::println!("AHCI: no sata device (degraded)");
            return;
        }
    };

    // Ensure PORTx.CMD.ST and FRE are set
    let cmd = abar_r(abar, po + PXCMD);
    if cmd & (CMD_ST | CMD_FRE) != (CMD_ST | CMD_FRE) {
        abar_w(abar, po + PXCMD, cmd | CMD_FRE | CMD_ST);
    }

    // Allocate CLB and FB if needed
    if abar_r(abar, po + PXCLB) == 0 {
        let clb = Box::leak(alloc::vec![0u8; 1024].into_boxed_slice());
        let fb = Box::leak(alloc::vec![0u8; 256].into_boxed_slice());
        let clbp = phys(clb.as_ptr());
        let fbp = phys(fb.as_ptr());
        abar_w(abar, po + PXCLB, clbp as u32);
        abar_w(abar, po + PXCLB + 4, (clbp >> 32) as u32);
        abar_w(abar, po + PXFB, fbp as u32);
        abar_w(abar, po + PXFB + 4, (fbp >> 32) as u32);
    }

    // READ SECTOR 1 (GPT header) to verify disk access
    let mut sec = alloc::vec![0u8; 512];
    match read_sector(abar, po, 1, &mut sec) {
        Ok(()) => {
            if &sec[0..8] == b"EFI PART" {
                crate::println!("AHCI-DISK-OK gpt");
            } else {
                crate::println!("AHCI-DISK-OK (no gpt sig)");
            }
        }
        Err(e) => {
            crate::println!("AHCI: read fail {} (degraded)", e);
        }
    }
}

fn read_sector(base: *mut u8, port: u32, lba: u64, buf: &mut [u8]) -> Result<(), &'static str> {
    // Build CFIS (Register H2D FIS, 64 bytes)
    let mut cfis = [0u8; 64];
    cfis[0] = 0x27; // FIS_TYPE_REG_H2D
    cfis[1] = 0x80; // C=1
    cfis[2] = 0x25; // COMMAND: READ DMA EXT
    cfis[4] = (lba & 0xFF) as u8;
    cfis[5] = ((lba >> 8) & 0xFF) as u8;
    cfis[6] = ((lba >> 16) & 0xFF) as u8;
    cfis[7] = 0x40 | (((lba >> 24) & 0x0F) as u8); // DEVICE: LBA bit
    cfis[8] = ((lba >> 32) & 0xFF) as u8;
    cfis[9] = ((lba >> 40) & 0xFF) as u8;
    cfis[10] = ((lba >> 48) & 0xFF) as u8;
    cfis[11] = ((lba >> 56) & 0xFF) as u8;
    cfis[12] = 1; // SECTOR COUNT (lower)
    cfis[13] = 0; // SECTOR COUNT (upper)

    // PRDT (1 entry, 16 bytes)
    let dba = phys(buf.as_ptr());
    let mut prdt = [0u8; 16];
    prdt[0..4].copy_from_slice(&(dba as u32).to_le_bytes());
    prdt[4..8].copy_from_slice(&((dba >> 32) as u32).to_le_bytes());
    prdt[12..16].copy_from_slice(&((buf.len() as u32).wrapping_sub(1)).to_le_bytes());

    // Command table: CFIS(64) + ACMD(16) + PRDT(16) = 96 bytes, aligned to 32
    let mut ct = Vec::with_capacity(128);
    ct.extend_from_slice(&cfis);
    ct.extend_from_slice(&[0u8; 16]); // ACMD
    ct.extend_from_slice(&prdt);
    ct.resize(128, 0);
    let ct_ptr = ct.leak().as_ptr();
    let ct_phys = phys(ct_ptr);

    // Update command header
    let clb_phys = abar_r(base, port + PXCLB) as u64
        | ((abar_r(base, port + PXCLBU) as u64) << 32);
    let clb_virt = (clb_phys + crate::heap::hhdm_offset() as u64) as *mut CmdHeader;
    unsafe {
        (*clb_virt).flags = (5 << 8) | 1; // CFL=5, PRDTL=1
        (*clb_virt).prdtl = 1;
        (*clb_virt).ctba = ct_phys as u32;
        (*clb_virt).ctbau = (ct_phys >> 32) as u32;
    }

    // Issue command
    abar_w(base, port + PXIS, 0xFFFFFFFF);
    abar_w(base, port + PXCI, 1);

    // Wait for completion
    for _ in 0..2_000_000 {
        if abar_r(base, port + PXCI) & 1 == 0 {
            let is = abar_r(base, port + PXIS);
            if is & (1 << 30) != 0 {
                return Err("ahci tfes");
            }
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err("ahci timeout")
}

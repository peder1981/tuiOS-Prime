//! Driver NVMe basico — Fase 4
//! Suporta leitura de setores

use core::ptr;

const NVME_BAR0: u64 = 0; // Será preenchido via PCI
const NVME_REG_CAP: u32 = 0x00;
const NVME_REG_MSIX: u32 = 0x08;
const NVME_REG_CC: u32 = 0x14;
const NVME_REG_CSTS: u32 = 0x1C;

const NVME_CMD_DWORD1_NSID: u32 = 1 << 0;

#[repr(C)]
struct NvmeCmd {
    opcode: u8,
    flags: u8,
    cid: u16,
    nsid: u32,
    cdw2: u32,
    cdw3: u32,
    mptr: u64,
    prp1: u64,
    prp2: u64,
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
    cdw13: u32,
    cdw14: u32,
    cdw15: u32,
}

const NVME_OP_SET_FEATURES: u8 = 0x09;
const NVME_OP_READ: u8 = 0x02;
const NVME_OP_WRITE: u8 = 0x01;

pub struct NvmeController {
    mmio: *mut u32,
    bar_size: usize,
}

unsafe impl Send for NvmeController {}

impl NvmeController {
    pub fn probe(_dev: &crate::pci::PciDev) -> Option<Self> {
        // TODO: Implementar em Fase 4
        None
    }

    pub fn read_sector(&self, _lba: u64, _buf: &mut [u8; 512]) -> Result<(), &'static str> {
        Err("nvme not implemented in Fase 4")
    }
}

pub fn init() {
    crate::println!("NVME: stub (Fase 4)");
}

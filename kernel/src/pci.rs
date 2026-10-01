use alloc::vec::Vec;
use x86_64::instructions::port::Port;

const CONF_ADDR: u16 = 0xCF8;
const CONF_DATA: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PciDev {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor: u16,
    pub device: u16,
    pub class: u8,
    pub subclass: u8,
}

fn cfg_addr(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    0x8000_0000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | (off as u32 & 0xFC)
}

pub fn read_u32(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let mut a: Port<u32> = Port::new(CONF_ADDR);
    let mut d: Port<u32> = Port::new(CONF_DATA);
    unsafe {
        a.write(cfg_addr(bus, dev, func, off));
        d.read()
    }
}

pub fn write_u32(bus: u8, dev: u8, func: u8, off: u8, val: u32) {
    let mut a: Port<u32> = Port::new(CONF_ADDR);
    let mut d: Port<u32> = Port::new(CONF_DATA);
    unsafe {
        a.write(cfg_addr(bus, dev, func, off));
        d.write(val)
    }
}

pub fn read_u16(b: u8, d: u8, f: u8, off: u8) -> u16 {
    (read_u32(b, d, f, off) >> ((off & 2) * 8)) as u16
}

pub fn enumerate() -> Vec<PciDev> {
    let mut out = Vec::new();
    for bus in 0..=255u16 {
        for dev in 0..32u8 {
            for func in 0..8u8 {
                let vd = read_u32(bus as u8, dev, func, 0x00);
                let vendor = (vd & 0xFFFF) as u16;
                if vendor == 0xFFFF {
                    if func == 0 {
                        break;
                    } else {
                        continue;
                    }
                }
                let device = (vd >> 16) as u16;
                let classreg = read_u32(bus as u8, dev, func, 0x08);
                out.push(PciDev {
                    bus: bus as u8,
                    dev,
                    func,
                    vendor,
                    device,
                    class: (classreg >> 24) as u8,
                    subclass: (classreg >> 16) as u8,
                });
                if func == 0 && (read_u32(bus as u8, dev, func, 0x0C) >> 16 & 0x80) == 0 {
                    break;
                }
            }
            if out.len() >= 32 {
                return out;
            }
        }
    }
    out
}

pub fn enable_bus_master(d: &PciDev) {
    let cmd = read_u32(d.bus, d.dev, d.func, 0x04);
    write_u32(d.bus, d.dev, d.func, 0x04, cmd | 0x07); // IO + MEM + BUSMASTER
}

pub fn bar(d: &PciDev, n: u8) -> (u64, bool) {
    let off = 0x10 + n * 4;
    let lo = read_u32(d.bus, d.dev, d.func, off);
    if lo & 0x1 == 0x1 {
        ((lo & 0xFFFF_FFFC) as u64, true)
    } else {
        let ty = (lo >> 1) & 0x3;
        if ty == 0x2 {
            let hi = read_u32(d.bus, d.dev, d.func, off + 4);
            (((hi as u64) << 32) | ((lo & 0xFFFF_FFF0) as u64), false)
        } else {
            ((lo & 0xFFFF_FFF0) as u64, false)
        }
    }
}

use crate::{print, println, serial};

pub fn run() -> ! {
    println!("TUIOS-PRIME SHELL (fase1)");
    println!("SHELL-OK");
    let mut buf = [0u8; 128];
    loop {
        print!("> ");
        let n = serial::read_line(&mut buf);
        let line: &[u8] = &buf[..n];
        if line == b"help" as &[u8] {
            println!("cmds: help echo time pci");
        } else if line == b"echo" as &[u8] {
            println!("echo...");
        } else if line == b"time" as &[u8] {
            println!("{} ms", crate::time::millis());
        } else if line == b"pci" as &[u8] {
            for d in crate::pci::enumerate() {
                println!("pci {:02x}:{:02x}.{} {:04x}:{:04x} class={:02x}:{:02x}",
                    d.bus, d.dev, d.func, d.vendor, d.device, d.class, d.subclass);
            }
        } else {
            println!("unknown cmd");
        }
    }
}

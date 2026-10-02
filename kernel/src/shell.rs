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
            println!("cmds: help echo time pci blk ls cat");
        } else if line == b"echo" as &[u8] {
            println!("echo...");
        } else if line == b"time" as &[u8] {
            println!("{} ms", crate::time::millis());
        } else if line.len() > 4 && &line[..4] == b"blk " as &[u8] {
            let mut lba = 0u64;
            let mut bad = false;
            for &c in &line[4..] {
                if !c.is_ascii_digit() { bad = true; break; }
                lba = lba * 10 + (c - b'0') as u64;
            }
            if bad { println!("bad lba"); } else { crate::blk::cmd_read(lba); }
        } else if line == b"ls" as &[u8] {
            crate::fs::cmd_ls();
        } else if line.len() > 4 && &line[..4] == b"cat " as &[u8] {
            match core::str::from_utf8(&line[4..]) {
                Ok(name) => crate::fs::cmd_cat(name.trim()),
                Err(_) => println!("bad name"),
            }
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

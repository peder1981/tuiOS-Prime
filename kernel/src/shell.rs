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
            println!("cmds: help echo time");
        } else if line == b"echo" as &[u8] {
            println!("echo...");
        } else if line == b"time" as &[u8] {
            println!("{} ms", crate::time::millis());
        } else {
            println!("unknown cmd");
        }
    }
}

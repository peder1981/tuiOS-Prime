use crate::{print, println, serial};

/// Comando advplc (ajuda; o compilador completo vive na ISO NixOS)
fn cmd_advplc() {
    println!("AdvPP - Compilador AdvPL/TLPP");
    println!("Uso: advplc <comando> <arquivo> [opcoes]");
    println!("");
    println!("Comandos:");
    println!("  run <arq>            Compila e executa");
    println!("  compile <arq> [-o <sai>]  Compila para bytecode");
    println!("  exec <bytecode>      Executa o bytecode");
    println!("  check <arq>          Valida a sintaxe");
    println!("  serve <arq> [--port <n>]  Modo web");
    println!("  build <arq> [-o <sai>] [--gui]  Gera executavel");
    println!("  debug <arq>          Servidor de depuracao DAP");
    println!("  ast <arq>            Exibe a AST");
    println!("  bytecode <arq>       Exibe o bytecode");
    println!("");
    println!("Nota: suporte completo na ISO NixOS (Fase 5)");
}

pub fn run() -> ! {
    println!("TUIOS-PRIME SHELL (fase1)");
    println!("SHELL-OK");
    let mut buf = [0u8; 128];
    loop {
        print!("> ");
        let n = serial::read_line(&mut buf);
        let line: &[u8] = &buf[..n];
        if line == b"help" as &[u8] {
            println!("comandos: help echo time pic pci blk ls cat net ping http advplc");
        } else if line == b"echo" as &[u8] {
            println!("eco...");
        } else if line == b"time" as &[u8] {
            println!("{} ms", crate::time::millis());
        } else if line == b"pic" as &[u8] {
            crate::arch::cmd_diag_pic_pit();
        } else if line.len() > 4 && &line[..4] == b"blk " as &[u8] {
            let mut lba = 0u64;
            let mut bad = false;
            for &c in &line[4..] {
                if !c.is_ascii_digit() { bad = true; break; }
                lba = lba * 10 + (c - b'0') as u64;
            }
            if bad { println!("lba invalido"); } else { crate::blk::cmd_read(lba); }
        } else if line == b"ls" as &[u8] {
            crate::fs::cmd_ls();
        } else if line.len() > 4 && &line[..4] == b"cat " as &[u8] {
            match core::str::from_utf8(&line[4..]) {
                Ok(name) => crate::fs::cmd_cat(name.trim()),
                Err(_) => println!("nome invalido"),
            }
        } else if line == b"net" as &[u8] {
            crate::println!("SHELL-CMD: net");
            match crate::net::STACK.lock().as_ref() {
                Some(s) => {
                    crate::println!("net {} ip={} gw={}", s.dev_name(), s.ip_str(), s.gw_str());
                    crate::println!("SHELL-NET-DONE");
                },
                None => crate::println!("net: indisponivel (degradado)"),
            }
        } else if line == b"ping" as &[u8] {
            crate::net::cmd_ping();
        } else if line == b"advplc" as &[u8] {
            cmd_advplc();
        } else if line == b"http" as &[u8] {
            crate::net::cmd_http();
        } else if line == b"pci" as &[u8] {
            for d in crate::pci::enumerate() {
                println!("pci {:02x}:{:02x}.{} {:04x}:{:04x} class={:02x}:{:02x}",
                    d.bus, d.dev, d.func, d.vendor, d.device, d.class, d.subclass);
            }
        } else {
            println!("comando desconhecido");
        }
    }
}

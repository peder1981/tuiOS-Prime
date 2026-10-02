//! FAT32 read-only mínimo (Fase 1): BPB + cadeia FAT + diretório raiz + leitura.
//! Sem LFN (short names), sem escrita (Fase 3). Sem dependências externas.
use alloc::string::String;
use alloc::vec::Vec;
use crate::blk::BlockDevice;
use spin::Mutex;

pub static MOUNTED: Mutex<bool> = Mutex::new(false);

#[derive(Clone, Copy)]
struct Geometry {
    bytes_per_sector: u32,
    sectors_per_cluster: u32,
    reserved: u32,
    num_fats: u32,
    fat_sectors: u32,
    root_cluster: u32,
}

static GEOM: Mutex<Option<Geometry>> = Mutex::new(None);

fn rd(lba: u64, buf: &mut [u8; 512]) -> Result<(), &'static str> {
    let mut g = crate::blk::VIRTIO_BLK.lock();
    g.as_mut().ok_or("nodev")?.read_block(lba, buf)
}

fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn parse_bpb() -> Result<Geometry, &'static str> {
    let mut s = [0u8; 512];
    rd(0, &mut s)?;
    if u16le(&s, 510) != 0xAA55 {
        return Err("no boot sig");
    }
    if &s[82..90] != b"FAT32   " {
        return Err("not fat32");
    }
    Ok(Geometry {
        bytes_per_sector: u16le(&s, 11) as u32,
        sectors_per_cluster: s[13] as u32,
        reserved: u16le(&s, 14) as u32,
        num_fats: s[16] as u32,
        fat_sectors: u32le(&s, 36),
        root_cluster: u32le(&s, 44),
    })
}

fn cluster_lba(g: &Geometry, cluster: u32) -> u64 {
    // assume 512 B/setor (validado: só prossegue se bytes_per_sector == 512)
    let data_start = g.reserved + g.num_fats * g.fat_sectors;
    (data_start + (cluster - 2) * g.sectors_per_cluster) as u64
}

fn fat_entry(g: &Geometry, cluster: u32) -> Result<u32, &'static str> {
    let off_bytes = cluster * 4;
    let lba = (g.reserved + off_bytes / 512) as u64;
    let mut s = [0u8; 512];
    rd(lba, &mut s)?;
    Ok(u32le(&s, (off_bytes % 512) as usize) & 0x0FFF_FFFF)
}

fn cluster_chain(g: &Geometry, start: u32, out: &mut Vec<u32>) -> Result<(), &'static str> {
    let mut c = start;
    loop {
        out.push(c);
        let n = fat_entry(g, c)?;
        if n >= 0x0FFF_FFF8 {
            return Ok(());
        }
        if n == 0 || n == 0x0FFF_FFF7 {
            return Err("bad cluster");
        }
        if out.len() > 4096 {
            return Err("chain too long");
        }
        c = n;
    }
}

#[derive(Clone)]
struct DirEnt {
    name: String,
    attr: u8,
    cluster: u32,
    size: u32,
}

fn short_name(raw: &[u8]) -> String {
    let mut s = String::new();
    let base = core::str::from_utf8(&raw[0..8]).unwrap_or("????????").trim_end();
    let ext = core::str::from_utf8(&raw[8..11]).unwrap_or("???").trim_end();
    s.push_str(base);
    if !ext.is_empty() {
        s.push('.');
        s.push_str(ext);
    }
    s
}

fn read_dir(g: &Geometry, start: u32) -> Result<Vec<DirEnt>, &'static str> {
    let mut clusters = Vec::new();
    cluster_chain(g, start, &mut clusters)?;
    let mut out = Vec::new();
    let mut sec = [0u8; 512];
    for &c in &clusters {
        for s_idx in 0..g.sectors_per_cluster {
            rd(cluster_lba(g, c) + s_idx as u64, &mut sec)?;
            for e in sec.chunks_exact(32) {
                if e[0] == 0x00 {
                    return Ok(out); // fim do diretório
                }
                if e[0] == 0xE5 {
                    continue; // deletado
                }
                let attr = e[11];
                if attr == 0x0F || attr == 0x08 {
                    continue; // LFN / label
                }
                let hi = u16le(e, 20) as u32;
                let lo = u16le(e, 26) as u32;
                out.push(DirEnt {
                    name: short_name(&e[0..11]),
                    attr,
                    cluster: (hi << 16) | lo,
                    size: u32le(e, 28),
                });
            }
        }
    }
    Ok(out)
}

fn read_file(g: &Geometry, ent: &DirEnt, out: &mut Vec<u8>) -> Result<(), &'static str> {
    if ent.cluster < 2 {
        return Err("empty cluster");
    }
    let mut clusters = Vec::new();
    cluster_chain(g, ent.cluster, &mut clusters)?;
    let mut sec = [0u8; 512];
    let mut left = ent.size as usize;
    for &c in &clusters {
        for s_idx in 0..g.sectors_per_cluster {
            if left == 0 {
                return Ok(());
            }
            rd(cluster_lba(g, c) + s_idx as u64, &mut sec)?;
            let n = core::cmp::min(512, left);
            out.extend_from_slice(&sec[..n]);
            left -= n;
        }
    }
    Ok(())
}

fn upper(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        o.push(c.to_ascii_uppercase());
    }
    o
}

pub fn mount() {
    match parse_bpb() {
        Ok(g) => {
            if g.bytes_per_sector != 512 {
                crate::println!("FS: sector size {} unsupported (degraded)", g.bytes_per_sector);
                return;
            }
            *GEOM.lock() = Some(g);
            *MOUNTED.lock() = true;
            crate::println!("FS-OK");
        }
        Err(_) => crate::println!("FS: no data disk (degraded)"),
    }
}

pub fn cmd_ls() {
    let g = match *GEOM.lock() {
        Some(g) => g,
        None => {
            crate::println!("FS-LS-FAIL not mounted");
            return;
        }
    };
    match read_dir(&g, g.root_cluster) {
        Ok(entries) => {
            for e in &entries {
                let tag = if e.attr & 0x10 != 0 { "<DIR>" } else { "     " };
                crate::println!("  {} {} {}", tag, e.size, e.name);
            }
            crate::println!("FS-LS-OK files={}", entries.len());
        }
        Err(e) => crate::println!("FS-LS-FAIL {}", e),
    }
}

pub fn cmd_cat(name: &str) {
    let want = upper(name.trim());
    let g = match *GEOM.lock() {
        Some(g) => g,
        None => {
            crate::println!("FS-CAT-FAIL not mounted");
            return;
        }
    };
    let entries = match read_dir(&g, g.root_cluster) {
        Ok(e) => e,
        Err(e) => {
            crate::println!("FS-CAT-FAIL {}", e);
            return;
        }
    };
    let ent = match entries.iter().find(|e| e.attr & 0x10 == 0 && upper(&e.name) == want) {
        Some(e) => e.clone(),
        None => {
            crate::println!("FS-CAT-FAIL not found");
            return;
        }
    };
    let mut data = Vec::new();
    match read_file(&g, &ent, &mut data) {
        Ok(()) => {
            crate::println!("{}", core::str::from_utf8(&data).unwrap_or("?bin?"));
            crate::println!("FS-CAT-OK");
        }
        Err(e) => crate::println!("FS-CAT-FAIL {}", e),
    }
}

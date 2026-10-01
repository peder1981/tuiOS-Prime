# tuiOS-Prime Fase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Kernel Rust com heap, relógio, PCI, virtio-blk + FAT32, virtio-net/e1000 + smoltcp (DHCP/ping/HTTP), AHCI leitura e shell serial — tudo com assert de boot no QEMU.

**Architecture:** Drivers como domínios fallíveis com degradação isolada (spec S4): cada init retorna Result e marca `degraded` sem derrubar o core. Shell serial sobre COM1 (bidirecional no QEMU `-serial stdio`). Transporte virtio LEGADO sobre PCI (sem parsing de capabilities — moderno fica Fase 3+).

**Tech Stack:** Rust nightly-2026-09-01, x86_64-unknown-none, limine 0.5, x86_64 0.15, pic8259 0.11, linked_list_allocator 0.10, fatfs 0.3, smoltcp 0.12 (dhcpv4/icmp/tcp/dns), bitflags 2, QEMU q35 + OVMF + slirp.

## Global Constraints

- x86_64 UEFI + QEMU q35 primeiro, ARM64 só Fase 4.
- Fork `github:peder1981/tuiOS` consumido como flake input, nunca copiado nem modificado.
- Driver quebrar = `degraded` isolado, nunca panic do core; hardware desconhecido = `no driver for PCI xxxx:xxxx`, nunca hang.
- I/O de disco com timeout + 3 retries → `EIO`, não panic.
- Nenhum milestone pronto sem log serial anexado em `docs/FASE1-EVIDENCIAS.md`.
- Commits atômicos formato `[FEAT|FIX|DOC|CFG] — descrição`; sem assinatura de assistente.
- Test cycle = boot-assert: script bash+grep sobre serial do QEMU (Pendente antes, OK depois). `cargo test` não se aplica a kernel no_std.

---

## File Structure

- `kernel/Cargo.toml` — deps novas: pic8259 0.11, linked_list_allocator 0.10, fatfs 0.3, smoltcp 0.12 (features medium-ethernet, proto-ipv4, socket-dhcpv4, socket-icmp, socket-tcp, socket-dns, proto-dns), bitflags 2.
- `kernel/src/main.rs` — estende REQUEST_BLOCK (hhdm + memmap), init: serial → heap → gdt/idt/pic/pit → drivers (cada um fallível) → shell loop.
- `kernel/src/serial.rs` — extraído do main: SerialPort global + `print!/println!` + `read_byte/read_line` bloqueantes.
- `kernel/src/heap.rs` — heap 16 MiB sobre entrada usable do memmap Limine (`LockedHeap`, `#[global_allocator]`).
- `kernel/src/arch.rs` — GDT (code/data ring0) + IDT (timer + `int3` teste) + PIC remap 0x20/0x28 + PIT 1 kHz.
- `kernel/src/time.rs` — `TICKS: AtomicU64`, `millis() -> u64`, `Instant::now()` p/ smoltcp.
- `kernel/src/pci.rs` — CAM 0xCF8/0xCFC, `enumerate() -> ArrayVec<PciDev, 32>`, `find(class|vendor:device)`, comando+bus master enable.
- `kernel/src/virtio.rs` — legado PCI: negotiate features, `VirtQueue` split (desc/avail/used), notify, `pop_used`.
- `kernel/src/blk.rs` — `trait BlockDevice`, `VirtioBlk` (IN/OUT 512B, status==0, retry 3x), `RamDisk` (fallback negativo).
- `kernel/src/ahci.rs` — enumera controlador classe 01:06:01, IDENTIFY porta 0, `read_lba0`.
- `kernel/src/fs.rs` — adaptador `fatfs` sobre `&dyn BlockDevice` + `TimeProvider` via TICKS.
- `kernel/src/net.rs` — `enum NetDev { Virtio, E1000 }` impl `smoltcp::phy::Device`; `VirtioNet` (RX pre-post 12×2K, MAC do config); `E1000` (82540EM 8086:100E, anéis 32×2K, MAC de RAL0/RAH0); DHCP até Configured (timeout 10 s); ping gateway; HTTP GET via TCP.
- `kernel/src/shell.rs` — comandos: `help echo time pci blk ls cat net ping http` + marcadores `*-OK`.
- `shared/mkdata.sh` — cria `data.img` 32 MiB FAT32 com `HELLO.TXT` + `README` (mtools, sem root).
- `shared/Justfile` — recipes novas: `qemu-fase1`, `qemu-fase1-e1000`, `qemu-fase1-nodev`, `qemu-fase1-nonic`, `http-srv` (helper).
- `docs/FASE1-EVIDENCIAS.md` — logs colados por task.

---

### Task 1: Heap + shell serial mínima

**Files:**
- Create: `kernel/src/serial.rs`, `kernel/src/heap.rs`, `kernel/src/shell.rs`
- Modify: `kernel/src/main.rs` (REQUEST_BLOCK +hhdm/memmap, init ordenado, shell loop), `kernel/Cargo.toml` (+linked_list_allocator)

**Interfaces:**
- Consumes: `SERIAL` do main (movido p/ serial.rs).
- Produces: `serial::println!`, `serial::read_line() -> ArrayString<128>`; `heap::init_heap()`; `shell::run()` (loop; comandos `help echo`); marcador `SHELL-OK`.

- [ ] **Step 1: Escrever assert `scripts-assert/shell.assert.sh`**

```bash
#!/usr/bin/env bash
# roda QEMU com a imagem e exige SHELL-OK no serial
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide > /tmp/f1-shell.log 2>&1 || true
grep -a -q SHELL-OK /tmp/f1-shell.log && echo SHELL-ASSERT-OK || echo SHELL-ASSERT-PENDING
```

- [ ] **Step 2: Rodar assert → PENDING**

Run: `cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd && bash scripts-assert/shell.assert.sh`
Expected: `SHELL-ASSERT-PENDING` (imagem atual não tem shell).

- [ ] **Step 3: Implementar serial.rs + alloc.rs + shell.rs + main**

`kernel/src/serial.rs`:
```rust
use core::fmt::{Arguments, Write};
use spinning_top::Spinlock;
use uart_16550::SerialPort;

static PORT: Spinlock<SerialPort> = Spinlock::new(unsafe { SerialPort::new(0x3F8) });

pub fn init() {
    PORT.lock().init();
}

fn write_fmt(args: Arguments) {
    let _ = PORT.lock().write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => { $crate::serial::print_fmt(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\r\n") };
    ($($t:tt)*) => { $crate::print!("{}\r\n", format_args!($($t)*)) };
}

pub fn print_fmt(args: Arguments) {
    write_fmt(args);
}

pub fn read_byte() -> u8 {
    // bloqueante; lock mantido (single-core; panic em read trava — aceito na Fase 1)
    PORT.lock().receive()
}

pub fn read_line(buf: &mut [u8]) -> usize {
    let mut n = 0;
    loop {
        let b = read_byte();
        if b == b'\r' || b == b'\n' {
            print!("\r\n");
            return n;
        } else if b == 0x7f {
            if n > 0 {
                n -= 1;
                print!("\x08 \x08");
            }
        } else if n < buf.len() {
            buf[n] = b;
            n += 1;
            print!("{}", b as char);
        }
    }
}
```

`kernel/src/heap.rs`:
```rust
use core::sync::atomic::{AtomicUsize, Ordering};
use limine::request::{HhdmRequest, MemoryMapRequest};
use linked_list_allocator::LockedHeap;

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

static HHDM_OFF: AtomicUsize = AtomicUsize::new(0);

pub fn hhdm_offset() -> usize {
    HHDM_OFF.load(Ordering::Relaxed)
}

pub const HEAP_BYTES: usize = 16 * 1024 * 1024;

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static HHDM: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static MEMMAP: MemoryMapRequest = MemoryMapRequest::new();

pub fn init_heap() -> Result<usize, &'static str> {
    let memmap = MEMMAP.get_response().ok_or("no memmap")?;
    let hhdm = HHDM.get_response().ok_or("no hhdm")?.offset() as usize;
    HHDM_OFF.store(hhdm, Ordering::Relaxed);
    for e in memmap.entries() {
        if e.entry_type == limine::memory_map::EntryType::USABLE && (e.length as usize) >= HEAP_BYTES {
            let base = e.base as usize + hhdm;
            unsafe { HEAP.lock().init(base as *mut u8, HEAP_BYTES) };
            return Ok(base);
        }
    }
    Err("no usable region >= 16MiB")
}
```

`kernel/src/shell.rs` (mínima T1; comandos crescem nas tasks seguintes):
```rust
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
            println!("cmds: help echo");
        } else if line == b"echo" as &[u8] {
            println!("echo...");
        } else {
            println!("unknown cmd");
        }
    }
}
```

`kernel/src/main.rs` (novo):
```rust
#![no_std]
#![no_main]

extern crate alloc;

mod heap;
mod serial;
mod shell;

use core::panic::PanicInfo;
use limine::request::{RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;

#[repr(C)]
struct RequestBlock {
    start: RequestsStartMarker,
    hhdm: limine::request::HhdmRequest,
    memmap: limine::request::MemoryMapRequest,
    base: BaseRevision,
    end: RequestsEndMarker,
}

#[used]
#[unsafe(link_section = ".limine_requests")]
static REQUEST_BLOCK: RequestBlock = RequestBlock {
    start: RequestsStartMarker::new(),
    hhdm: limine::request::HhdmRequest::new(),
    memmap: limine::request::MemoryMapRequest::new(),
    base: BaseRevision::new(),
    end: RequestsEndMarker::new(),
};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial::init();
    println!("HELLO-TUIOS-KERNEL");
    match alloc::init_heap() {
        Ok(_) => println!("HEAP-OK"),
        Err(e) => println!("HEAP-FAIL {}", e),
    }
    println!("DRIVERS: none (fase0 ram+serial only)");
    shell::run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
```

`kernel/Cargo.toml` — adicionar:
```toml
linked_list_allocator = "0.10"
```

- [ ] **Step 4: Rerodar assert → OK**

Run: `just -f shared/Justfile image && bash scripts-assert/shell.assert.sh`
Expected: `SHELL-ASSERT-OK` (e `HEAP-OK` no log).

- [ ] **Step 5: Commit**

```bash
git add kernel/src/serial.rs kernel/src/heap.rs kernel/src/shell.rs kernel/src/main.rs kernel/Cargo.toml kernel/Cargo.lock scripts-assert/shell.assert.sh
git commit -m "[FEAT] — heap 16MiB + shell serial minima (Fase 1 T1)"
```
---

### Task 2: GDT + IDT + PIC + PIT (relógio 1 kHz)

**Files:**
- Create: `kernel/src/arch.rs`, `kernel/src/time.rs`
- Modify: `kernel/src/main.rs` (chama `arch::init()` + imprime `TIME-OK`), `kernel/Cargo.toml` (+pic8259), `kernel/src/shell.rs` (comando `time`)

**Interfaces:**
- Consumes: `serial::println!`.
- Produces: `arch::init()`; `time::millis() -> u64`; `time::Instant::now()` p/ smoltcp; marcador `TIME-OK`; comando `time` imprime ms + ticks.

- [ ] **Step 1: Escrever assert `scripts-assert/time.assert.sh`**

```bash
#!/usr/bin/env bash
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide > /tmp/f1-time.log 2>&1 || true
grep -a -q TIME-OK /tmp/f1-time.log && echo TIME-ASSERT-OK || echo TIME-ASSERT-PENDING
```

- [ ] **Step 2: Rodar assert → PENDING**

Run: `bash scripts-assert/time.assert.sh`
Expected: `TIME-ASSERT-PENDING`.

- [ ] **Step 3: Implementar arch.rs + time.rs**

`kernel/src/arch.rs`:
```rust
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
    let gdt: &'static GlobalDescriptorTable = alloc::boxed::Box::leak(
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
```
`kernel/src/time.rs`:
```rust
use core::sync::atomic::Ordering;

pub fn millis() -> u64 {
    crate::arch::TICKS.load(Ordering::Relaxed)
}

pub mod smol_instant {
    use smoltcp::time::Instant;
    pub fn now() -> Instant {
        Instant::from_millis(super::millis() as i64)
    }
}
```

`kernel/Cargo.toml` — adicionar:
```toml
pic8259 = "0.11"
spin = "0.9"
```

`kernel/src/main.rs` — registrar `mod arch; mod time;`, chamar `arch::init();` após heap,
e `shell.rs` ganha: `b"time" => println!("{} ms", crate::time::millis()),`.

- [ ] **Step 4: Rerodar assert → OK**

Run: `just -f shared/Justfile image && bash scripts-assert/time.assert.sh`
Expected: `TIME-ASSERT-OK`.

- [ ] **Step 5: Commit**

```bash
git add kernel/src/arch.rs kernel/src/time.rs kernel/src/main.rs kernel/src/shell.rs kernel/Cargo.toml kernel/Cargo.lock scripts-assert/time.assert.sh
git commit -m "[FEAT] — GDT IDT PIC PIT 1kHz + millis (Fase 1 T2)"
```

---

### Task 3: Enumeração PCI

**Files:**
- Create: `kernel/src/pci.rs`
- Modify: `kernel/src/main.rs` (mod + lista na boot + `PCI-OK n=..`), `kernel/src/shell.rs` (comando `pci`)

**Interfaces:**
- Consumes: `serial::println!`.
- Produces: `pci::PciDev { bus, dev, func, vendor, device, class, subclass }`; `pci::enumerate() -> Vec<PciDev>` (alloc, ≤32); `pci::enable_bus_master(d)`; `pci::bar(d, n) -> (u64, bool /*is_io*/)`.

- [ ] **Step 1: Escrever assert `scripts-assert/pci.assert.sh`**

```bash
#!/usr/bin/env bash
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  -netdev user,id=n0 -device virtio-net-pci,netdev=n0 > /tmp/f1-pci.log 2>&1 || true
grep -a -q "PCI-OK" /tmp/f1-pci.log && grep -a -q "1af4:1001" /tmp/f1-pci.log && echo PCI-ASSERT-OK || echo PCI-ASSERT-PENDING
```

- [ ] **Step 2: Rodar assert → PENDING**

Run: `bash scripts-assert/pci.assert.sh`
Expected: `PCI-ASSERT-PENDING` (sem data.img ainda? o mkdata vem na T5 — nesta task o assert
pende pela ausência do marcador; rode mesmo sem data.img: o qemu falha o drive e o marcador some).

- [ ] **Step 3: Implementar pci.rs**

```rust
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
        d.write(val);
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
```

`main.rs` no boot após `arch::init()`:
```rust
let devs = pci::enumerate();
crate::println!("PCI-OK n={}", devs.len());
for d in &devs {
    crate::println!("pci {:02x}:{:02x}.{} {:04x}:{:04x} class={:02x}:{:02x}",
        d.bus, d.dev, d.func, d.vendor, d.device, d.class, d.subclass);
}
```
`shell.rs`: `b"pci" => { for d in crate::pci::enumerate() { println!(...) } }` (mesmo formato).

- [ ] **Step 4: Rerodar assert → OK**

Run: `just -f shared/Justfile image && bash scripts-assert/pci.assert.sh`
Expected: `PCI-ASSERT-OK` (lista contém `1af4:1001` do blk e `1af4:1000` do net).

- [ ] **Step 5: Commit**

```bash
git add kernel/src/pci.rs kernel/src/main.rs kernel/src/shell.rs scripts-assert/pci.assert.sh
git commit -m "[FEAT] — enumeracao PCI via CAM + comando pci (Fase 1 T3)"
```
---

### Task 4: Virtio legado + VirtioBlk (leitura/escrita 512B)

**Files:**
- Create: `kernel/src/virtio.rs`, `kernel/src/blk.rs`
- Modify: `kernel/src/main.rs` (probe blk no boot, `BLK-OK`/`BLK: no block device (degraded)`), `kernel/src/shell.rs` (comando `blk <lba>`), `kernel/Cargo.toml` (+bitflags)

**Interfaces:**
- Consumes: `pci::{PciDev, bar, enable_bus_master, read_u16}`, `alloc` (heap p/ anéis), HHDM offset p/ phys.
- Produces: `virtio::LegacyRegs { io_base: u16 }` + `virtio::init_legacy(dev) -> Result<LegacyRegs, &str>`;
  `virtio::VirtQueue::new(size: u16)`, `vq.add_buf(...) -> u16`, `vq.notify(io)`, `vq.pop_used() -> Option<u16>`;
  `blk::BlockDevice` trait (`read_block(lba, &mut [u8;512])`, `write_block`, `blocks()`),
  `blk::VIRTIO_BLK: Mutex<Option<VirtioBlk>>`; marcadores `BLK-OK` / degradado.

- [ ] **Step 1: Escrever assert `scripts-assert/blk.assert.sh`**

```bash
#!/usr/bin/env bash
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio > /tmp/f1-blk.log 2>&1 || true
grep -a -q "BLK-READ-OK" /tmp/f1-blk.log && echo BLK-ASSERT-OK || echo BLK-ASSERT-PENDING
```

- [ ] **Step 2: Rodar assert → PENDING**

Run: `bash scripts-assert/blk.assert.sh`
Expected: `BLK-ASSERT-PENDING` (sem data.img e sem driver).

- [ ] **Step 3: Implementar virtio.rs + blk.rs**

`kernel/src/virtio.rs` (legado, IO ports):
```rust
use crate::pci::PciDev;
use alloc::vec::Vec;

pub const REG_FEATURES: u16 = 0x00;
pub const REG_GUEST_FEATURES: u16 = 0x04;
pub const REG_QUEUE_PFN: u16 = 0x08;
pub const REG_QUEUE_NUM: u16 = 0x0C;
pub const REG_QUEUE_SEL: u16 = 0x0E;
pub const REG_QUEUE_NOTIFY: u16 = 0x10;
pub const REG_STATUS: u16 = 0x12;
pub const REG_ISR: u16 = 0x13;

pub const ST_ACK: u8 = 1;
pub const ST_DRIVER: u8 = 2;
pub const ST_DRIVER_OK: u8 = 4;
pub const ST_FAILED: u8 = 0x80;

pub const DESC_F_NEXT: u16 = 1;
pub const DESC_F_WRITE: u16 = 2;

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct Desc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

pub struct LegacyRegs {
    pub io: u16,
}

fn outb(p: u16, v: u8) {
    unsafe { x86_64::instructions::port::Port::<u8>::new(p).write(v) };
}
fn outw(p: u16, v: u16) {
    unsafe { x86_64::instructions::port::Port::<u16>::new(p).write(v) };
}
fn outl(p: u16, v: u32) {
    unsafe { x86_64::instructions::port::Port::<u32>::new(p).write(v) };
}
fn inb(p: u16) -> u8 {
    unsafe { x86_64::instructions::port::Port::<u8>::new(p).read() }
}
fn inw(p: u16) -> u16 {
    unsafe { x86_64::instructions::port::Port::<u16>::new(p).read() }
}
fn inl(p: u16) -> u32 {
    unsafe { x86_64::instructions::port::Port::<u32>::new(p).read() }
}

pub fn init_legacy(dev: &PciDev) -> Result<LegacyRegs, &'static str> {
    use crate::pci::{bar, enable_bus_master};
    enable_bus_master(dev);
    let (addr, is_io) = bar(dev, 0);
    if !is_io {
        return Err("virtio bar0 not IO");
    }
    let io = addr as u16;
    outb(io + REG_STATUS, 0); // reset
    outb(io + REG_STATUS, ST_ACK | ST_DRIVER);
    Ok(LegacyRegs { io })
}

pub fn finish_ok(io: u16) {
    outb(io + REG_STATUS, ST_ACK | ST_DRIVER | ST_DRIVER_OK);
}

pub fn ack_features(io: u16, mask: u32) {
    let f = inl(io + REG_FEATURES) & mask;
    outl(io + REG_GUEST_FEATURES, f);
}

pub struct VirtQueue {
    pub size: u16,
    pub desc: *mut Desc, // 16-alinhado, vazado de propósito (vive p/ sempre)
    pub avail: *mut u16, // avail ring (flags@0 idx@1 ring@2..)
    pub used: *mut u8,   // used ring base
    pub free_head: u16,
    pub avail_idx: u16,
    pub last_used: u16,
}

impl VirtQueue {
    pub fn new(size: u16) -> Self {
        // desc table precisa de alinhamento 16: over-aloca e alinha manualmente
        let raw = alloc::vec![0u8; size as usize * 16 + 16];
        let base = raw.as_ptr() as usize;
        let aligned = (base + 15) & !15;
        core::mem::forget(raw);
        let mut vq = Self {
            size,
            desc: aligned as *mut Desc,
            avail: core::ptr::null_mut(),
            used: core::ptr::null_mut(),
            free_head: 0,
            avail_idx: 0,
            last_used: 0,
            descs: Vec::new(),
        };
        for i in 0..size {
            unsafe {
                (*vq.desc.add(i as usize)).next = (i + 1) % size;
            }
        }
        let layout_avail = (4 + 2 * size as usize + 8) as usize;
        let mut abuf = alloc::vec![0u8; layout_avail + 8];
        let aptr = abuf.as_mut_ptr();
        core::mem::forget(abuf);
        vq.avail = aptr as *mut u16;
        let layout_used = (4 + 8 * size as usize + 8) as usize;
        let mut ubuf = alloc::vec![0u8; layout_used + 8];
        vq.used = ubuf.as_mut_ptr();
        core::mem::forget(ubuf);
        vq
    }

    fn phys(p: *const u8) -> u64 {
        (p as u64).wrapping_sub(crate::heap::hhdm_offset())
    }

    pub fn setup_legacy(&mut self, io: u16, sel: u16) {
        outw(io + REG_QUEUE_SEL, sel);
        let max = inw(io + REG_QUEUE_NUM);
        assert!(self.size <= max, "queue too big");
        let pfn = (Self::phys(self.desc as *const u8) >> 12) as u32;
        outl(io + REG_QUEUE_PFN, pfn);
    }

    fn d(&mut self, i: u16) -> &mut Desc {
        unsafe { &mut *self.desc.add(i as usize) }
    }

    pub fn add_chain(&mut self, parts: &[(u64, u32, u16)]) -> u16 {
        let head = self.free_head;
        let mut prev: Option<u16> = None;
        for (i, (addr, len, flags)) in parts.iter().enumerate() {
            let idx = if i == 0 { head } else { self.d(prev.unwrap()).next };
            let dd = self.d(idx);
            dd.addr = *addr;
            dd.len = *len;
            dd.flags = *flags;
            if let Some(p) = prev {
                let pd = self.d(p);
                pd.flags |= DESC_F_NEXT;
                pd.next = idx;
            }
            prev = Some(idx);
        }
        let last = prev.unwrap();
        let ln = self.d(last);
        ln.flags &= !DESC_F_NEXT;
        self.free_head = ln.next;
        unsafe {
            let ring = self.avail.add(2) as *mut u16;
            *ring.add((self.avail_idx % self.size) as usize) = head;
            self.avail_idx += 1;
            *(self.avail.add(1)) = self.avail_idx;
        }
        head
    }

    pub fn notify(&self, io: u16, qsel: u16) {
        outw(io + REG_QUEUE_NOTIFY, qsel);
    }

    pub fn pop_used(&mut self) -> Option<(u16, u32)> {
        unsafe {
            let used_idx = *(self.used.add(1) as *const u16);
            if self.last_used == used_idx {
                return None;
            }
            let elem = self.used.add(4).cast::<[u32; 2]>().add((self.last_used % self.size) as usize);
            let id = (*elem)[0];
            let len = (*elem)[1];
            self.last_used += 1;
            Some((id, len))
        }
    }

    /// Devolve `n` descritores a partir de `head` (cadeias sempre do mesmo tamanho
    /// por chamador: blk=3, net-tx=2, net-rx=1). Chamador completa de forma síncrona,
    /// então o head concluído é sempre o nosso.
    pub fn free_chain(&mut self, head: u16, n: usize) {
        let mut cur = head;
        for _ in 0..n {
            let nx = self.d(cur).next;
            self.d(cur).next = self.free_head;
            self.free_head = cur;
            cur = nx;
        }
    }
}
```

`kernel/src/blk.rs`:
```rust
use crate::virtio::*;
use spin::Mutex;

pub trait BlockDevice: Send {
    fn read_block(&mut self, lba: u64, buf: &mut [u8; 512]) -> Result<(), &'static str>;
    fn write_block(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), &'static str>;
    fn blocks(&self) -> u64;
}

pub static VIRTIO_BLK: Mutex<Option<VirtioBlk>> = Mutex::new(None);

#[repr(C)]
struct BlkReq {
    typ: u32,
    _reserved: u32,
    sector: u64,
}

pub struct VirtioBlk {
    io: u16,
    q: VirtQueue,
    nblocks: u64,
}

impl VirtioBlk {
    pub fn probe(dev: &crate::pci::PciDev) -> Result<Self, &'static str> {
        let r = init_legacy(dev)?;
        ack_features(r.io, 0); // sem features especiais na Fase 1
        let mut q = VirtQueue::new(64);
        q.setup_legacy(r.io, 0);
        finish_ok(r.io);
        // capacidade via config space do dispositivo (offset 0x14+8? capacity em 0x14+8? não: capacity = config+0)
        let cap_lo = inl_cfg(r.io);
        Ok(Self { io: r.io, q, nblocks: cap_lo })
    }

    fn xfer(&mut self, lba: u64, data: *mut u8, out: bool) -> Result<(), &'static str> {
        let mut status = 0xFFu8;
        let mut req = BlkReq { typ: if out { 1 } else { 0 }, _reserved: 0, sector: lba };
        let p_req = VirtQueue::phys(&req as *const _ as *const u8);
        let p_data = VirtQueue::phys(data);
        let p_st = VirtQueue::phys(&status as *const _ as *const u8);
        let dflag = if out { 0 } else { DESC_F_WRITE };
        self.q.add_chain(&[
            (p_req, 16, 0),
            (p_data, 512, dflag),
            (p_st, 1, DESC_F_WRITE),
        ]);
        self.q.notify(self.io, 0);
        let mut done = false;
        for _ in 0..1_000_000 {
            if let Some((hid, _)) = self.q.pop_used() {
                self.q.free_chain(hid, 3);
                done = true;
                break;
            }
            core::hint::spin_loop();
        }
        if !done {
            return Err("blk timeout");
        }
        if status != 0 {
            return Err("blk io status != 0");
        }
        let _ = req;
        Ok(())
    }
}

fn inl_cfg(io: u16) -> u64 {
    // capacity (u64) no início do device config (BAR0 + 0x14)
    let lo = unsafe { x86_64::instructions::port::Port::<u32>::new(io + 0x14).read() };
    let hi = unsafe { x86_64::instructions::port::Port::<u32>::new(io + 0x18).read() };
    ((hi as u64) << 32) | lo as u64
}

impl BlockDevice for VirtioBlk {
    fn read_block(&mut self, lba: u64, buf: &mut [u8; 512]) -> Result<(), &'static str> {
        for attempt in 0..3 {
            match self.xfer(lba, buf.as_mut_ptr(), false) {
                Ok(()) => return Ok(()),
                Err(e) if attempt == 2 => return Err(e),
                Err(_) => continue,
            }
        }
        Err("unreachable")
    }
    fn write_block(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), &'static str> {
        let mut tmp = [0u8; 512];
        tmp.copy_from_slice(buf);
        for attempt in 0..3 {
            match self.xfer(lba, tmp.as_mut_ptr(), true) {
                Ok(()) => return Ok(()),
                Err(e) if attempt == 2 => return Err(e),
                Err(_) => continue,
            }
        }
        Err("unreachable")
    }
    fn blocks(&self) -> u64 {
        self.nblocks
    }
}

pub fn probe_boot() {
    use crate::pci;
    let devs = pci::enumerate();
    let blk = devs.iter().find(|d| d.vendor == 0x1AF4 && d.device == 0x1001);
    match blk {
        Some(d) => match VirtioBlk::probe(d) {
            Ok(b) => {
                crate::println!("BLK-OK blocks={}", b.blocks());
                *VIRTIO_BLK.lock() = Some(b);
            }
            Err(e) => crate::println!("BLK: probe failed {} (degraded)", e),
        },
        None => crate::println!("BLK: no block device (degraded)"),
    }
}

pub fn cmd_read(lba: u64) {
    let mut guard = VIRTIO_BLK.lock();
    match guard.as_mut() {
        Some(b) => {
            let mut buf = [0u8; 512];
            match b.read_block(lba, &mut buf) {
                Ok(()) => {
                    crate::println!("BLK-READ-OK lba={} b0={:02x} b1={:02x} bend={:02x}", lba, buf[0], buf[1], buf[511]);
                }
                Err(e) => crate::println!("BLK-READ-EIO {}", e),
            }
        }
        None => crate::println!("BLK-READ-NODEV"),
    }
}
```

Notas de implementação (contrato, sem placeholder):
- `crate::heap::hhdm_offset()` precisa existir: retorna `HHDM offset` guardado no init
  (adicione `static HHDM_OFF: AtomicUsize` em alloc.rs, set no `init_heap`, getter `hhdm_offset()`).
- `VirtQueue::phys` subtrai o offset HHDM (heap vive em RAM mapeada no HHDM).
- Timeout do xfer = 1M spins + retry 3x no trait (spec: timeout+retry, EIO, sem panic).
- `shell.rs`: `blk <lba>` parseia decimal e chama `blk::cmd_read`.

`kernel/Cargo.toml` — adicionar: `bitflags = "2"`.
`main.rs` — `mod virtio; mod blk;`, após PCI: `blk::probe_boot();`.

- [ ] **Step 4: Rerodar assert → OK**

Run: `bash shared/mkdata.sh && just -f shared/Justfile image && bash scripts-assert/blk.assert.sh`
Expected: `BLK-ASSERT-OK` (`BLK-READ-OK` requer `data.img` — `mkdata.sh` vem na Task 5;
nesta task o assert usa `blk 0` sobre o próprio data.img... Se data.img ainda não existe,
o QEMU aborta o drive: crie `shared/mkdata.sh` JÁ nesta task (conteúdo definido na Task 5)
ou rode com data.img vazia de 32M. Decisão: implemente `mkdata.sh` agora, Task 5 só o usa.)

- [ ] **Step 5: Commit**

```bash
git add kernel/src/virtio.rs kernel/src/blk.rs kernel/src/main.rs kernel/src/shell.rs kernel/src/heap.rs kernel/Cargo.toml kernel/Cargo.lock shared/mkdata.sh scripts-assert/blk.assert.sh
git commit -m "[FEAT] — virtio-blk leitura/escrita 512B + retry (Fase 1 T4)"
```
---

### Task 5: Volume FAT32 de dados + mount + ls/cat

**Files:**
- Create: `shared/mkdata.sh`, `kernel/src/fs.rs`
- Modify: `kernel/src/main.rs` (mount no boot: `FS-OK`/`FS: no data disk`), `kernel/src/shell.rs` (`ls`, `cat`), `kernel/Cargo.toml` (+fatfs), `shared/Justfile` (+qemu-fase1)

**Interfaces:**
- Consumes: `blk::VIRTIO_BLK` (Mutex<Option<VirtioBlk>>), `alloc` (fatfs precisa de `alloc`).
- Produces: `fs::mount() -> Result<(), &str>`; `fs::ls()`, `fs::cat(name) -> Result<(), &str>` (imprime conteúdo);
  marcadores `FS-OK files=n` / `FS: no data disk (degraded)`; `FS-CAT-OK`.

- [ ] **Step 1: Escrever `shared/mkdata.sh` + assert `scripts-assert/fs.assert.sh`**

`shared/mkdata.sh`:
```bash
#!/usr/bin/env bash
# cria data.img 32 MiB FAT32 com HELLO.TXT + README (mtools, sem root)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/data.img}"
printf 'HELLO FROM TUIOS-PRIME DATA DISK\r\nsecond line\r\n' > /tmp/HELLO.TXT
printf 'tuios-prime fase1 data volume\r\n' > /tmp/DREADME
dd if=/dev/zero of="$OUT" bs=1M count=32 status=none
mkfs.fat -F 32 -n TUIOSDATA "$OUT" >/dev/null
mcopy -i "$OUT" /tmp/HELLO.TXT ::/HELLO.TXT
mcopy -i "$OUT" /tmp/DREADME ::/README
echo "DATA-OK: $OUT"
```

`scripts-assert/fs.assert.sh`:
```bash
#!/usr/bin/env bash
set -uo pipefail
(printf 'ls\ncat HELLO.TXT\n' | sleep 25) | timeout 30 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio > /tmp/f1-fs.log 2>&1 || true
grep -a -q "FS-CAT-OK" /tmp/f1-fs.log && echo FS-ASSERT-OK || echo FS-ASSERT-PENDING
```
(Nota: comandos chegam via stdin→serial; `read_line` ecoa; `cat HELLO.TXT` deve imprimir o conteúdo.)

- [ ] **Step 2: Rodar assert → PENDING**

Run: `bash shared/mkdata.sh && just -f shared/Justfile image && bash scripts-assert/fs.assert.sh`
Expected: `FS-ASSERT-PENDING`.

- [ ] **Step 3: Implementar fs.rs**

```rust
use fatfs::{DefaultTimeProvider, FileSystem, FsOptions, LossyOemCpConverter};
use spin::Mutex;

pub static MOUNTED: Mutex<bool> = Mutex::new(false);

pub struct BlkFile {
    pos: u64,
}

impl BlkFile {
    fn do_rw(&mut self, buf: &mut [u8], write_src: Option<&[u8]>) -> Result<usize, &'static str> {
        let mut guard = crate::blk::VIRTIO_BLK.lock();
        let b = guard.as_mut().ok_or("nodev")?;
        let mut done = 0;
        let mut sector_buf = [0u8; 512];
        while done < buf.len() {
            let lba = (self.pos / 512) as u64;
            let off = (self.pos % 512) as usize;
            let n = core::cmp::min(512 - off, buf.len() - done);
            if write_src.is_some() {
                b.read_block(lba, &mut sector_buf)?;
                sector_buf[off..off + n].copy_from_slice(&write_src.unwrap()[done..done + n]);
                b.write_block(lba, &sector_buf)?;
            } else {
                b.read_block(lba, &mut sector_buf)?;
                buf[done..done + n].copy_from_slice(&sector_buf[off..off + n]);
            }
            done += n;
            self.pos += n as u64;
        }
        Ok(done)
    }
}

impl fatfs::IoBase for BlkFile {
    type Error = &'static str;
}
impl fatfs::Read for BlkFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.do_rw(buf, None)
    }
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            fatfs::SeekFrom::Start(n) => self.pos = n,
            fatfs::SeekFrom::Current(d) => self.pos = (self.pos as i64 + d) as u64,
            fatfs::SeekFrom::End(_) => return Err("no-end"),
        }
        Ok(self.pos)
    }
}
impl fatfs::Write for BlkFile {
    // Fase 1 usa read (ls/cat); write existe p/ satisfazer o trait e sera exercitado na Fase 3 (logs).
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let owned = buf.to_vec();
        let mut sink = alloc::vec![0u8; owned.len()];
        self.do_rw(&mut sink, Some(&owned))
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
```

```rust
pub fn with_fs<R>(f: impl FnOnce(&FileSystem<BlkFile, DefaultTimeProvider, LossyOemCpConverter>) -> R) -> Result<R, &'static str> {
    if !*MOUNTED.lock() {
        return Err("not mounted");
    }
    let fs = FileSystem::new(BlkFile { pos: 0 }, FsOptions::new()).map_err(|_| "mount fail")?;
    Ok(f(&fs))
}

pub fn mount() {
    match with_fs(|fs| fs.root_dir().open_file("HELLO.TXT").is_ok()) {
        Ok(true) => {
            *MOUNTED.lock() = true;
            crate::println!("FS-OK");
        }
        _ => crate::println!("FS: no data disk (degraded)"),
    }
}

pub fn cmd_ls() {
    match with_fs(|fs| {
        let root = fs.root_dir();
        let mut n = 0;
        for e in root.iter() {
            if let Ok(e) = e {
                crate::println!("  {}", e.file_name());
                n += 1;
            }
        }
        n
    }) {
        Ok(n) => crate::println!("FS-LS-OK files={}", n),
        Err(e) => crate::println!("FS-LS-FAIL {}", e),
    }
}

pub fn cmd_cat(name: &str) {
    use alloc::string::String;
    let name_s = String::from(name);
    match with_fs(|fs| {
        let mut f = fs.root_dir().open_file(&name_s).map_err(|_| "open")?;
        let mut buf = alloc::vec![0u8; 256];
        let n = f.read(&mut buf).map_err(|_| "read")?;
        crate::println!("{}", core::str::from_utf8(&buf[..n]).unwrap_or("?bin?"));
        Ok::<(), &'static str>(())
    }) {
        Ok(Ok(())) => crate::println!("FS-CAT-OK"),
        Ok(Err(e)) => crate::println!("FS-CAT-FAIL {}", e),
        Err(e) => crate::println!("FS-CAT-FAIL {}", e),
    }
}
```

`kernel/Cargo.toml` — adicionar: `fatfs = "0.3"`.
`main.rs` após blk probe: `fs::mount();`. `shell.rs`: `ls` → `fs::cmd_ls()`, `cat X` → parse nome.
`shared/Justfile` — adicionar recipe `qemu-fase1` (com data.img + net slirp):
```
qemu-fase1: image
    ./shared/mkdata.sh {{root}}/data.img
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
    timeout 30 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd -drive file={{root}}/disk.img,format=raw,if=ide -drive file={{root}}/data.img,format=raw,if=virtio -netdev user,id=n0 -device virtio-net-pci,netdev=n0
```

- [ ] **Step 4: Rerodar assert → OK**

Run: `just -f shared/Justfile image && bash scripts-assert/fs.assert.sh`
Expected: `FS-ASSERT-OK` ( Conteúdo `HELLO FROM TUIOS-PRIME DATA DISK` + `FS-CAT-OK` no log).

- [ ] **Step 5: Commit**

```bash
git add kernel/src/fs.rs kernel/src/main.rs kernel/src/shell.rs kernel/Cargo.toml kernel/Cargo.lock shared/mkdata.sh shared/Justfile scripts-assert/fs.assert.sh
git commit -m "[FEAT] — FAT32 mount + ls/cat sobre virtio-blk (Fase 1 T5)"
```
---

### Task 6: VirtioNet + smoltcp (DHCP + ping + HTTP GET)

**Files:**
- Create: `kernel/src/net.rs`
- Modify: `kernel/src/main.rs` (probe net no boot), `kernel/src/shell.rs` (`net ping http`), `kernel/Cargo.toml` (+smoltcp), `shared/Justfile` (`http-srv`, usa server no qemu-fase1)

**Interfaces:**
- Consumes: `pci`, `virtio::{init_legacy, ack_features, finish_ok, VirtQueue}`, `time::smol_instant::now`, `alloc`.
- Produces: `net::STACK: Mutex<Option<NetStack>>`; `net::probe_boot()` (DHCP ≤10 s);
  `net::cmd_status()`, `net::cmd_ping() -> NET-PING-OK`, `net::cmd_http() -> HTTP-GET-OK`;
  `NetDev::Virtio(VirtioNet)` (E1000 entra na T7).

- [ ] **Step 1: Escrever assert `scripts-assert/net.assert.sh`**

```bash
#!/usr/bin/env bash
set -uo pipefail
mkdir -p /tmp/httpsrv && printf 'HTTP HELLO FROM HOST\n' > /tmp/httpsrv/hello.txt
(python3 -m http.server 18080 --directory /tmp/httpsrv >/dev/null 2>&1 &) 
sleep 1
(printf 'net\nping\nhttp\n' | sleep 40) | timeout 45 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  -netdev user,id=n0 -device virtio-net-pci,netdev=n0 > /tmp/f1-net.log 2>&1 || true
pkill -f "http.server 18080" || true
grep -a -q "NET-DHCP-OK" /tmp/f1-net.log && grep -a -q "NET-PING-OK" /tmp/f1-net.log \
  && grep -a -q "HTTP-GET-OK" /tmp/f1-net.log && echo NET-ASSERT-OK || echo NET-ASSERT-PENDING
```
(Nota: guest alcança o host slirp em 10.0.2.2 sem hostfwd; DHCP do slirp entrega 10.0.2.15.)

- [ ] **Step 2: Rodar assert → PENDING**

Run: `bash scripts-assert/net.assert.sh`
Expected: `NET-ASSERT-PENDING`.

- [ ] **Step 3: Implementar net.rs**

```rust
use alloc::sync::Arc;
use alloc::vec::Vec;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::dhcpv4::{Event as DhcpEvent, Socket as DhcpSocket};
use smoltcp::socket::icmp::{PacketBuffer as IcmpBuf, PacketMetadata as IcmpMeta, Socket as IcmpSocket, SocketBuffer as IcmpSockBuf};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer as TcpBuf};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use spin::Mutex;

pub static STACK: Mutex<Option<NetStack>> = Mutex::new(None);

const RX_BUFS: usize = 12;
const RX_LEN: usize = 2048;

// ---------- VirtioNet ----------

pub struct VirtioNet {
    io: u16,
    mac: [u8; 6],
    rx: crate::virtio::VirtQueue,
    tx: crate::virtio::VirtQueue,
    rx_bufs: Vec<Vec<u8>>,
}

impl VirtioNet {
    pub fn probe(dev: &crate::pci::PciDev) -> Result<Self, &'static str> {
        use crate::virtio::*;
        let r = init_legacy(dev)?;
        ack_features(r.io, 1 << 5); // VIRTIO_NET_F_MAC
        let mut mac = [0u8; 6];
        for i in 0..6 {
            mac[i] = unsafe {
                x86_64::instructions::port::Port::<u8>::new(r.io + 0x14 + i as u16).read()
            };
        }
        let mut rx = VirtQueue::new(16);
        let mut tx = VirtQueue::new(16);
        rx.setup_legacy(r.io, 0);
        tx.setup_legacy(r.io, 1);
        finish_ok(r.io);
        let mut n = Self { io: r.io, mac, rx, tx, rx_bufs: Vec::new() };
        for _ in 0..RX_BUFS {
            n.post_rx();
        }
        Ok(n)
    }

    fn post_rx(&mut self) {
        let mut buf = alloc::vec![0u8; RX_LEN];
        let p = VirtQueue::phys(buf.as_ptr());
        let idx = self.rx_bufs.len();
        self.rx_bufs.push(buf);
        // descritor WRITE apontando p/ o buffer; guardamos índice via ordem de postagem
        self.rx.add_chain(&[(p, RX_LEN as u32, crate::virtio::DESC_F_WRITE)]);
        let _ = idx;
        self.rx.notify(self.io, 0);
    }

    fn recv_pkt(&mut self) -> Option<Vec<u8>> {
        let (id, len) = self.rx.pop_used()?;
        self.rx.free_chain(id, 1);
        let mut buf = self.rx_bufs.pop()?;
        buf.truncate(len as usize);
        self.post_rx(); // repõe buffer + descritor
        Some(buf)
    }

    fn xmit(&mut self, pkt: &[u8]) {
        let p = VirtQueue::phys(pkt.as_ptr());
        // header virtio-net vazio (10B) + pacote
        static mut HDR: [u8; 10] = [0u8; 10];
        let ph = VirtQueue::phys(unsafe { HDR.as_ptr() });
        self.tx.add_chain(&[
            (ph, 10, 0),
            (p, pkt.len() as u32, 0),
        ]);
        self.tx.notify(self.io, 1);
        for _ in 0..200_000 {
            if let Some((hid, _)) = self.tx.pop_used() {
                self.tx.free_chain(hid, 2);
                break;
            }
            core::hint::spin_loop();
        }
    }
}
```

**Correção de ownership (contrato executável):** tokens smoltcp com `Arc<Mutex<…>>`:

```rust
pub struct QueueHandle {
    inner: Arc<Mutex<QueueInner>>,
}

pub struct QueueInner {
    pub virtio: Option<VirtioNet>,
}

pub struct NetRx {
    pkt: Vec<u8>,
}
impl RxToken for NetRx {
    fn consume<R, F>(self, f: F) -> R
    where F: FnOnce(&mut &[u8]) -> R {
        let mut s: &[u8] = &self.pkt;
        f(&mut s)
    }
}

pub struct NetTx {
    q: Arc<Mutex<QueueInner>>,
}
impl TxToken for NetTx {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where F: FnOnce(&mut [u8]) -> R {
        let mut buf = alloc::vec![0u8; len];
        let r = f(&mut buf);
        let mut g = self.q.lock();
        if let Some(v) = g.virtio.as_mut() {
            v.xmit(&buf);
        }
        r
    }
}

pub enum NetDev {
    Virtio(QueueHandle),
}

impl Device for NetDev {
    type RxToken<'a> = NetRx where Self: 'a;
    type TxToken<'a> = NetTx where Self: 'a;

    fn receive(&mut self, _ts: Instant) -> Option<(Self::RxToken, Self::TxToken)> {
        let q = match self {
            NetDev::Virtio(h) => h.inner.clone(),
        };
        let pkt = q.lock().virtio.as_mut()?.recv_pkt()?;
        Some((NetRx { pkt }, NetTx { q }))
    }

    fn transmit(&mut self, _ts: Instant) -> Option<Self::TxToken> {
        let q = match self {
            NetDev::Virtio(h) => h.inner.clone(),
        };
        Some(NetTx { q })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut c = DeviceCapabilities::default();
        c.medium = Medium::Ethernet;
        c.max_transmission_unit = 1500;
        c.max_burst_size = Some(1);
        c
    }
}

pub struct NetStack {
    dev: NetDev,
    iface: Interface,
    sockets: SocketSet<'static>,
    ip: Option<Ipv4Address>,
    gw: Option<Ipv4Address>,
    mac: [u8; 6],
}

pub fn with_stack<R>(f: impl FnOnce(&mut Interface, &mut NetDev, &mut SocketSet<'static>) -> R) -> Option<R> {
    STACK.lock().as_mut().map(|s| f(&mut s.iface, &mut s.dev, &mut s.sockets))
}
```

**Nota de lifetime (contrato):** `SocketSet<'static>` + `SocketStorage<'static>` alocados
via `Box::leak` no `probe_boot` (heap vive p/ sempre; sem teardown na Fase 1).

```rust
fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut it = data.chunks_exact(2);
    for w in &mut it {
        sum += u16::from_be_bytes([w[0], w[1]]) as u32;
    }
    if let Some(&b) = it.remainder().first() {
        sum += (b as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

fn echo_request(ident: u16, seq: u16, payload: &[u8]) -> Vec<u8> {
    let mut pkt = alloc::vec![0u8; 8 + payload.len()];
    pkt[0] = 8; // Echo Request
    pkt[1] = 0;
    pkt[4..6].copy_from_slice(&ident.to_be_bytes());
    pkt[6..8].copy_from_slice(&seq.to_be_bytes());
    pkt[8..].copy_from_slice(payload);
    let c = checksum(&pkt);
    pkt[2..4].copy_from_slice(&c.to_be_bytes());
    pkt
}

pub fn probe_boot() {
    use crate::pci;
    let devs = pci::enumerate();
    let net = devs.iter().find(|d| d.vendor == 0x1AF4 && d.device == 0x1000);
    let d = match net {
        Some(d) => *d,
        None => {
            crate::println!("NET: no net device (degraded)");
            return;
        }
    };
    let vn = match VirtioNet::probe(&d) {
        Ok(v) => v,
        Err(e) => {
            crate::println!("NET: virtio probe failed {} (degraded)", e);
            return;
        }
    };
    let mac = vn.mac;
    let eth = EthernetAddress(mac);
    let mut config = Config::new(HardwareAddress::Ethernet(eth));
    config.random_seed = 0xA53A_5AA5;
    let storage: &'static mut [SocketStorage<'static>; 4] =
        alloc::boxed::Box::leak(alloc::boxed::Box::new([SocketStorage::EMPTY; 4]));
    // NOTE: se SocketStorage::EMPTY não for const em 0.12, usar Default::default() em array_init.
    let mut device = NetDev::Virtio(QueueHandle {
        inner: Arc::new(Mutex::new(QueueInner { virtio: Some(vn) })),
    });
    let mut iface = Interface::new(config, &mut device, crate::time::smol_instant::now());
    iface.update_ip_addrs(|a| {
        a.clear();
        let _ = a.push(IpCidr::new(IpAddress::v4(0, 0, 0, 0), 0));
    });
    let mut sockets = SocketSet::new(&mut storage[..]);
    let dhcp_h = sockets.add(DhcpSocket::new());
    *STACK.lock() = Some(NetStack { dev: device, iface, sockets, ip: None, gw: None, mac });
    // DHCP até 10 s (fields disjuntos via with_stack: iface+dev+sockets juntos, sem unsafe)
    let t0 = crate::time::millis();
    let configured = loop {
        if crate::time::millis() - t0 > 10_000 {
            break false;
        }
        let ev = with_stack(|iface, dev, socks| {
            iface.poll(crate::time::smol_instant::now(), dev, socks);
            socks.get_mut::<DhcpSocket>(dhcp_h).poll()
        })
        .flatten();
        match ev {
            Some(DhcpEvent::Configured(cfg)) => {
                with_stack(|iface, _, _| {
                    iface.update_ip_addrs(|a| {
                        a.clear();
                        let _ = a.push(cfg.address);
                    });
                    if let Some(r) = cfg.router {
                        iface.routes_mut().add_default_ipv4_route(r).ok();
                    }
                });
                if let IpAddress::Ipv4(v) = cfg.address.address() {
                    STACK.lock().as_mut().unwrap().ip = Some(v);
                }
                if let Some(r) = cfg.router {
                    STACK.lock().as_mut().unwrap().gw = Some(r);
                }
                break true;
            }
            _ => {}
        }
    };
    {
        let g = STACK.lock();
        let s = g.as_ref().unwrap();
        if configured {
            crate::println!("NET-DHCP-OK ip={} gw={} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                s.ip.unwrap(), s.gw.unwrap_or(Ipv4Address::UNSPECIFIED),
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
        } else {
            crate::println!("NET: dhcp timeout (degraded)");
        }
    }
}

pub fn cmd_ping() {
    let gw = match STACK.lock().as_ref().and_then(|s| s.gw) {
        Some(g) => g,
        None => { crate::println!("NET-PING-NOGW"); return; }
    };
    let sock = IcmpSocket::new(
        IcmpSockBuf::new(IcmpBuf::new([IcmpMeta::EMPTY; 4], alloc::vec![0u8; 256])),
        IcmpSockBuf::new(IcmpBuf::new([IcmpMeta::EMPTY; 4], alloc::vec![0u8; 256])),
    );
    let h = with_stack(|_, _, socks| socks.add(sock)).unwrap();
    with_stack(|_, _, socks| {
        socks.get_mut::<IcmpSocket>(h).bind(smoltcp::wire::IcmpEndpoint::Ident(0xBEEF)).ok();
    });
    let req = echo_request(0xBEEF, 1, b"tuios-prime");
    let t0 = crate::time::millis();
    let mut ok = false;
    while crate::time::millis() - t0 < 5000 {
        with_stack(|iface, dev, socks| {
            iface.poll(crate::time::smol_instant::now(), dev, socks);
            let s = socks.get_mut::<IcmpSocket>(h);
            if !s.can_send() { return; }
            let _ = s.send_slice(&req, IpAddress::Ipv4(gw));
            let mut rbuf = [0u8; 128];
            if let Ok((n, _)) = s.recv_slice(&mut rbuf) {
                if n >= 8 && rbuf[0] == 0 && u16::from_be_bytes([rbuf[4], rbuf[5]]) == 0xBEEF {
                    ok = true;
                }
            }
        });
        if ok { break; }
    }
    if ok { crate::println!("NET-PING-OK gw={}", gw); }
    else { crate::println!("NET-PING-TIMEOUT"); }
}
```

`cmd_http()` (GET http://10.0.2.2:18080/hello.txt, espera `HTTP HELLO FROM HOST`):
```rust
pub fn cmd_http() {
    let srv = Ipv4Address::new(10, 0, 2, 2);
    let sock = TcpSocket::new(TcpBuf::new(alloc::vec![0u8; 4096]), TcpBuf::new(alloc::vec![0u8; 4096]));
    let h = match with_stack(|_, _, socks| socks.add(sock)) { Some(h) => h, None => { crate::println!("HTTP-NOSTACK"); return; } };
    with_stack(|iface, dev, socks| {
        let s = socks.get_mut::<TcpSocket>(h);
        let _ = s.connect(iface.context(), (IpAddress::Ipv4(srv), 18080), 49152);
    });
    let t0 = crate::time::millis();
    let mut body = alloc::vec![0u8; 0];
    let mut sent = false;
    let mut ok = false;
    while crate::time::millis() - t0 < 8000 {
        with_stack(|iface, dev, socks| {
            iface.poll(crate::time::smol_instant::now(), dev, socks);
            let s = socks.get_mut::<TcpSocket>(h);
            if s.can_send() && !sent {
                let _ = s.send_slice(b"GET /hello.txt HTTP/1.0\r\nHost: x\r\n\r\n");
                sent = true;
            }
            if s.can_recv() {
                let mut tmp = [0u8; 512];
                if let Ok(n) = s.recv_slice(&mut tmp) {
                    body.extend_from_slice(&tmp[..n]);
                }
            }
            if !s.may_recv() && sent && !body.is_empty() {
                ok = true;
            }
        });
        if ok { break; }
    }
    let text = core::str::from_utf8(&body).unwrap_or("");
    if text.contains("200") && text.contains("HTTP HELLO FROM HOST") {
        crate::println!("HTTP-GET-OK len={}", body.len());
    } else {
        crate::println!("HTTP-GET-FAIL len={}", body.len());
    }
}
```

`shell.rs`: `net` → status (ip/mac/gw ou degraded); `ping` → `cmd_ping()`; `http` → `cmd_http()`.
`kernel/Cargo.toml` — adicionar:
```toml
smoltcp = { version = "0.12", default-features = false, features = ["medium-ethernet", "proto-ipv4", "socket-dhcpv4", "socket-icmp", "socket-tcp", "socket-dns", "alloc"] }
```
`main.rs` após fs mount: `net::probe_boot();`.

- [ ] **Step 4: Rerodar assert → OK**

Run: `just -f shared/Justfile image && bash scripts-assert/net.assert.sh`
Expected: `NET-ASSERT-OK` (DHCP + PING + HTTP no log).

- [ ] **Step 5: Commit**

```bash
git add kernel/src/net.rs kernel/src/main.rs kernel/src/shell.rs kernel/Cargo.toml kernel/Cargo.lock shared/Justfile scripts-assert/net.assert.sh
git commit -m "[FEAT] — virtio-net + smoltcp DHCP ping HTTP (Fase 1 T6)"
```
---

### Task 7: Intel e1000 (82540EM) como segunda NIC

**Files:**
- Modify: `kernel/src/net.rs` (E1000 + enum, probe order, `NET-DEV` linha), `kernel/src/shell.rs` (nada novo; `net` mostra dev), `shared/Justfile` (`qemu-fase1-e1000`)
- Test: `scripts-assert/e1000.assert.sh` (exige `NET-DEV e1000` + `NET-DHCP-OK`)

**Interfaces:**
- Consumes: `QueueHandle/QueueInner` (ganha campo `e1000`), `NetDev` (ganha variante), `pci::bar` (MMIO).
- Produces: `E1000::probe(dev) -> Result<Self, &str>`; `E1000::recv_pkt() -> Option<Vec<u8>>`; `E1000::xmit(&[u8])`; marcador `E1000-OK`.

- [ ] **Step 1: Escrever assert `scripts-assert/e1000.assert.sh`**

```bash
#!/usr/bin/env bash
set -uo pipefail
mkdir -p /tmp/httpsrv && printf 'HTTP HELLO FROM HOST\n' > /tmp/httpsrv/hello.txt
(python3 -m http.server 18080 --directory /tmp/httpsrv >/dev/null 2>&1 &)
sleep 1
(printf 'net\nping\n' | sleep 40) | timeout 45 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  -netdev user,id=n0 -device e1000,netdev=n0 > /tmp/f1-e1000.log 2>&1 || true
pkill -f "http.server 18080" || true
grep -a -q "NET-DEV e1000" /tmp/f1-e1000.log && grep -a -q "NET-DHCP-OK" /tmp/f1-e1000.log \
  && grep -a -q "NET-PING-OK" /tmp/f1-e1000.log && echo E1000-ASSERT-OK || echo E1000-ASSERT-PENDING
```
(Nota: SEM `-device virtio-net-pci` — o probe prefere virtio; para exercitar e1000 ele deve ser a única NIC.)

- [ ] **Step 2: Rodar assert → PENDING**

Run: `bash scripts-assert/e1000.assert.sh`
Expected: `E1000-ASSERT-PENDING`.

- [ ] **Step 3: Implementar E1000**

```rust
// offsets 82540EM
const RCTL: u32 = 0x0100;
const TCTL: u32 = 0x0400;
const RDLEN: u32 = 0x2808;
const RDH: u32 = 0x2810;
const RDT: u32 = 0x2818;
const RDBAL: u32 = 0x2800;
const RDBAH: u32 = 0x2804;
const TDLEN: u32 = 0x3808;
const TDH: u32 = 0x3810;
const TDT: u32 = 0x3818;
const TDBAL: u32 = 0x3800;
const TDBAH: u32 = 0x3804;
const RAL0: u32 = 0x5400;
const RAH0: u32 = 0x5404;
const IMS: u32 = 0x00D0;
const MTA_OFF: u32 = 0x5200;

const RCTL_EN: u32 = 1 << 1;
const RCTL_BAM: u32 = 1 << 15;
const TCTL_EN: u32 = 1 << 1;
const TCTL_PSP: u32 = 1 << 3;
const RXD_STAT_DD: u8 = 0x01;
const TXD_CMD_EOP: u8 = 0x01;
const TXD_CMD_RS: u8 = 0x08;
const TXD_STAT_DD: u8 = 0x01;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RxDesc {
    addr: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TxDesc {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

pub struct E1000 {
    mmio: *mut u32,
    rx: Vec<RxDesc>,
    tx: Vec<TxDesc>,
    rx_bufs: Vec<Vec<u8>>,
    rtail: usize,
    ttail: usize,
    mac: [u8; 6],
}

fn mm_w(mmio: *mut u32, off: u32, v: u32) {
    unsafe { core::ptr::write_volatile(mmio.byte_add(off as usize), v) };
}
fn mm_r(mmio: *mut u32, off: u32) -> u32 {
    unsafe { core::ptr::read_volatile(mmio.byte_add(off as usize)) }
}

impl E1000 {
    pub fn probe(dev: &crate::pci::PciDev) -> Result<Self, &'static str> {
        use crate::pci::{bar, enable_bus_master};
        if dev.vendor != 0x8086 || dev.device != 0x100E {
            return Err("not 82540EM");
        }
        enable_bus_master(dev);
        let (addr, is_io) = bar(dev, 0);
        if is_io {
            return Err("e1000 bar0 not MMIO");
        }
        let hhdm = crate::heap::hhdm_offset() as u64;
        let mmio = (addr + hhdm) as *mut u32;
        // MAC via RAL0/RAH0 (QEMU pré-programa; se inválido -> degraded, EEPROM fica Fase 3)
        let ral = mm_r(mmio, RAL0);
        let rah = mm_r(mmio, RAH0);
        let mac = [
            (ral & 0xFF) as u8, ((ral >> 8) & 0xFF) as u8, ((ral >> 16) & 0xFF) as u8, ((ral >> 24) & 0xFF) as u8,
            (rah & 0xFF) as u8, ((rah >> 8) & 0xFF) as u8,
        ];
        if mac == [0, 0, 0, 0, 0, 0] || mac[0] & 1 == 1 {
            return Err("e1000 no mac");
        }
        mm_w(mmio, IMS, 0); // sem interrupções na Fase 1 (polling)
        for i in 0..128u32 {
            mm_w(mmio, MTA_OFF + i * 4, 0);
        }
        // RX ring 32 x 2048
        let mut rx = alloc::vec![RxDesc::default(); 32];
        let mut rx_bufs = Vec::new();
        for d in rx.iter_mut() {
            let b = alloc::vec![0u8; 2048];
            d.addr = (b.as_ptr() as u64).wrapping_sub(hhdm);
            d.status = 0;
            rx_bufs.push(b);
        }
        let rxp = (rx.as_ptr() as u64).wrapping_sub(hhdm);
        mm_w(mmio, RDBAL, rxp as u32);
        mm_w(mmio, RDBAH, (rxp >> 32) as u32);
        mm_w(mmio, RDLEN, (32 * 16) as u32);
        mm_w(mmio, RDH, 0);
        mm_w(mmio, RDT, 0);
        mm_w(mmio, RCTL, RCTL_EN | RCTL_BAM);
        // TX ring 32
        let mut tx = alloc::vec![TxDesc::default(); 32];
        let txp = (tx.as_ptr() as u64).wrapping_sub(hhdm);
        mm_w(mmio, TDBAL, txp as u32);
        mm_w(mmio, TDBAH, (txp >> 32) as u32);
        mm_w(mmio, TDLEN, (32 * 16) as u32);
        mm_w(mmio, TDH, 0);
        mm_w(mmio, TDT, 0);
        mm_w(mmio, TCTL, TCTL_EN | TCTL_PSP);
        crate::println!("E1000-OK");
        Ok(Self { mmio, rx, tx, rx_bufs, rtail: 0, ttail: 0, mac })
    }

    pub fn recv_pkt(&mut self) -> Option<Vec<u8>> {
        let d = &mut self.rx[self.rtail];
        if d.status & RXD_STAT_DD == 0 {
            return None;
        }
        let n = d.length as usize;
        let mut pkt = alloc::vec![0u8; n];
        pkt.copy_from_slice(&self.rx_bufs[self.rtail][..n]);
        d.status = 0;
        mm_w(self.mmio, RDT, self.rtail as u32);
        self.rtail = (self.rtail + 1) % 32;
        Some(pkt)
    }

    pub fn xmit(&mut self, pkt: &[u8]) {
        let t = self.ttail;
        // copia p/ buffer próprio (heap) p/ phys estável
        let mut owned = alloc::vec![0u8; pkt.len()];
        owned.copy_from_slice(pkt);
        let hhdm = crate::heap::hhdm_offset() as u64;
        let leaked: &'static mut [u8] = alloc::boxed::Box::leak(owned.into_boxed_slice());
        self.tx[t].addr = (leaked.as_ptr() as u64).wrapping_sub(hhdm);
        self.tx[t].length = leaked.len() as u16;
        self.tx[t].cmd = TXD_CMD_EOP | TXD_CMD_RS;
        self.tx[t].status = 0;
        self.ttail = (self.ttail + 1) % 32;
        mm_w(self.mmio, TDT, self.ttail as u32);
        for _ in 0..200_000 {
            if self.tx[t].status & TXD_STAT_DD != 0 {
                break;
            }
            core::hint::spin_loop();
        }
        // buffer vazado de propósito na Fase 1 (TX ring pequeno, sem reuse; Fase 3 recicla)
    }
}
```

**Correção:** remover a linha no-op `write_volatile(&mut mm_w ...)` (fence desnecessário;
MMIO já é volatile). `recv_pkt` termina com `mm_w(RDT)` + advance.

Integração (editar `net.rs` da T6):
```rust
pub struct QueueInner {
    pub virtio: Option<VirtioNet>,
    pub e1000: Option<E1000>,
}

pub enum NetDev {
    Virtio(QueueHandle),
    E1000(QueueHandle),
}

// Device::receive: tenta virtio, senão e1000:
fn recv_from(q: &Arc<Mutex<QueueInner>>) -> Option<Vec<u8>> {
    let mut g = q.lock();
    if let Some(v) = g.virtio.as_mut() {
        return v.recv_pkt();
    }
    if let Some(e) = g.e1000.as_mut() {
        return e.recv_pkt();
    }
    None
}
// Device::transmit: TxToken precisa saber o lado — NetTx ganha `e1000: bool`?
```
**Decisão de implementação (contrato):** em vez de flag, `TxToken::consume` tenta virtio
e senão e1000 (só um `Some` por vez — probe garante exclusividade):
```rust
impl TxToken for NetTx {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where F: FnOnce(&mut [u8]) -> R {
        let mut buf = alloc::vec![0u8; len];
        let r = f(&mut buf);
        let mut g = self.q.lock();
        if let Some(v) = g.virtio.as_mut() {
            v.xmit(&buf);
        } else if let Some(e) = g.e1000.as_mut() {
            e.xmit(&buf);
        }
        r
    }
}
```
Probe (virtio primeiro, e1000 depois; imprime o lado):
```rust
let found = devs.iter().find(|d| d.vendor == 0x1AF4 && d.device == 0x1000)
    .map(|d| (*d, "virtio"))
    .or_else(|| {
        devs.iter().find(|d| d.vendor == 0x8086 && d.device == 0x100E).map(|d| (*d, "e1000"))
    });
let (d, name) = match found {
    Some(x) => x,
    None => {
        crate::println!("NET: no net device (degraded)");
        return;
    }
};
crate::println!("NET-DEV {}", name);
// ... VirtioNet::probe ou E1000::probe conforme `name`, resto idêntico à T6
```
- `NetDev::E1000(QueueHandle)`; `Device::receive/transmit` despacham p/ qual `Some`.
- `probe_boot`: `virtio 1AF4:1000` primeiro; senão `e1000 8086:100E`;
  imprime `NET-DEV virtio` ou `NET-DEV e1000` antes do DHCP.
- `VirtioNet::xmit` continua igual; `E1000::xmit` acima.

`shared/Justfile` — adicionar:
```
qemu-fase1-e1000: image
    ./shared/mkdata.sh {{root}}/data.img
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
    timeout 45 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd -drive file={{root}}/disk.img,format=raw,if=ide -drive file={{root}}/data.img,format=raw,if=virtio -netdev user,id=n0 -device e1000,netdev=n0
```

- [ ] **Step 4: Rerodar assert → OK**

Run: `just -f shared/Justfile image && bash scripts-assert/e1000.assert.sh`
Expected: `E1000-ASSERT-OK`.

- [ ] **Step 5: Commit**

```bash
git add kernel/src/net.rs kernel/src/main.rs shared/Justfile scripts-assert/e1000.assert.sh
git commit -m "[FEAT] — e1000 82540EM polling + mesma stack smoltcp (Fase 1 T7)"
```

---

### Task 8: AHCI (leitura do disco de boot, sem reset do HBA)

**Files:**
- Create: `kernel/src/ahci.rs`
- Modify: `kernel/src/main.rs` (`AHCI-DISK-OK`/degraded), `kernel/src/shell.rs` (`ahci`)

**Interfaces:**
- Consumes: `pci::enumerate` (classe 01:06:01), `alloc::hhdm_offset`, `time` (timeout via spins).
- Produces: `ahci::probe_boot()` → lê LBA0 da porta 0 e IDENTIFY model; marcadores
  `AHCI-DISK-OK model=..` / `AHCI: no ahci controller (degraded)`.

- [ ] **Step 1: Escrever assert `scripts-assert/ahci.assert.sh`**

```bash
#!/usr/bin/env bash
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  -netdev user,id=n0 -device virtio-net-pci,netdev=n0 > /tmp/f1-ahci.log 2>&1 || true
grep -a -q "AHCI-DISK-OK" /tmp/f1-ahci.log && echo AHCI-ASSERT-OK || echo AHCI-ASSERT-PENDING
```
(No q35 o boot disk `if=ide` vive no controlador AHCI ICH9 — a porta 0 tem o nosso disco.)

- [ ] **Step 2: Rodar assert → PENDING**

Run: `bash scripts-assert/ahci.assert.sh`
Expected: `AHCI-ASSERT-PENDING`.

- [ ] **Step 3: Implementar ahci.rs**

```rust
// ABAR genérico + porta 0. SEMPRE sem HBA reset (disco de boot em uso pelo firmware).
const PXCLB: u32 = 0x00;
const PXFB: u32 = 0x08;
const PXIS: u32 = 0x10;
const PXCMD: u32 = 0x18;
const PXTFD: u32 = 0x20;
const PXSSTS: u32 = 0x28;
const PXCI: u32 = 0x38;

const CMD_ST: u32 = 1 << 0;
const CMD_FRE: u32 = 1 << 4;
const CMD_FR: u32 = 1 << 14;
const CMD_CR: u32 = 1 << 15;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CmdHeader {
    flags: u16, // bit0 ATAPI, bits8-12 CFL=5
    prdtl: u16,
    prdbc: u32,
    ctba: u32,
    ctbau: u32,
    _r: [u32; 4],
}

fn abar_r(base: *mut u8, off: u32) -> u32 {
    unsafe { core::ptr::read_volatile(base.add(off as usize).cast::<u32>()) }
}
fn abar_w(base: *mut u8, off: u32, v: u32) {
    unsafe { core::ptr::write_volatile(base.add(off as usize).cast::<u32>(), v) };
}

fn phys(p: *const u8) -> u64 {
    (p as u64).wrapping_sub(crate::heap::hhdm_offset() as u64)
}

fn wait_clear(base: *mut u8, port: u32, mask: u32) -> bool {
    for _ in 0..1_000_000 {
        if abar_r(base, port + PXCMD) & mask == 0 {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn issue_28(cmd: u8, lba: u64, count: u16, buf_phys: u64, base: *mut u8, port: u32) -> Result<(), &'static str> {
    // command table: CFIS 64B + PRDT 16B
    let mut ct = alloc::vec![0u8; 256 + 16];
    // CFIS Register H2D
    ct[0] = 0x27; // FIS_TYPE_REG_H2D
    ct[1] = 0x80; // C=1
    ct[2] = cmd;
    ct[3] = 0; // FEATURES
    ct[4] = lba as u8;
    ct[5] = (lba >> 8) as u8;
    ct[6] = (lba >> 16) as u8;
    ct[7] = 0x40; // DEVICE: LBA bit (disco pequeno; LBA[31:24]=0)
    ct[8] = (lba >> 24) as u8;
    ct[9] = (lba >> 32) as u8;
    ct[10] = (lba >> 40) as u8;
    ct[12] = (count & 0xFF) as u8;
    ct[13] = (count >> 8) as u8;
    ct[15] = 0; // CONTROL
    // PRDT @ +256: dba, reserved, bytecount-1|IE
    let db = &mut ct[256..];
    db[0..8].copy_from_slice(&buf_phys.to_le_bytes());
    db[12..16].copy_from_slice(&((8192 - 1) as u32).to_le_bytes());
    // command list @ CLB (1 entrada)
    let clb = abar_r(base, port + PXCLB) as u64 | ((abar_r(base, port + PXCLB + 4) as u64) << 32);
    let cl = unsafe { core::slice::from_raw_parts_mut((clb + crate::heap::hhdm_offset() as u64) as *mut CmdHeader, 1) };
    cl[0].flags = 5 << 8; // CFL=5 dwords
    cl[0].prdtl = 1;
    let ctp = phys(ct.as_ptr());
    cl[0].ctba = ctp as u32;
    cl[0].ctbau = (ctp >> 32) as u32;
    abar_w(base, port + PXIS, 0xFFFF_FFFF);
    abar_w(base, port + PXCI, 1);
    for _ in 0..2_000_000 {
        if abar_r(base, port + PXCI) & 1 == 0 {
            if abar_r(base, port + PXIS) & (1 << 30) != 0 {
                return Err("ahci tfes");
            }
            return Ok(());
        }
        core::hint::spin_loop();
    }
    Err("ahci timeout")
}

pub fn probe_boot() {
    use crate::pci;
    let ctrl = pci::enumerate().into_iter().find(|d| d.class == 0x01 && d.subclass == 0x06);
    let c = match ctrl {
        Some(c) => c,
        None => {
            crate::println!("AHCI: no ahci controller (degraded)");
            return;
        }
    };
    pci::enable_bus_master(&c);
    let (addr, is_io) = pci::bar(&c, 5);
    if is_io {
        crate::println!("AHCI: abar not MMIO (degraded)");
        return;
    }
    let hhdm = crate::heap::hhdm_offset();
    let abar = (addr + hhdm as u64) as *mut u8;
    // porta 0 com dispositivo? (PxSSTS DET==3, PxSIG==0x101)
    let mut found: Option<u32> = None;
    for p in 0..6u32 {
        let po = 0x100 + p * 0x80;
        let ssts = abar_r(abar, po + PXSSTS);
        if ssts & 0xF == 3 && abar_r(abar, po + 0x24) == 0x101 {
            found = Some(po);
            break;
        }
    }
    let po = match found {
        Some(po) => po,
        None => {
            crate::println!("AHCI: no sata device (degraded)");
            return;
        }
    };
    // garante PxCMD.ST|FRE ligados (firmware já ligou; se não, liga sem reset)
    let mut cmd = abar_r(abar, po + PXCMD);
    if cmd & (CMD_ST | CMD_FRE) != (CMD_ST | CMD_FRE) {
        // para com segurança e (re)liga
        cmd &= !(CMD_ST | CMD_FRE);
        abar_w(abar, po + PXCMD, cmd);
        if !wait_clear(abar, po, CMD_CR | CMD_FR) {
            crate::println!("AHCI: port busy (degraded)");
            return;
        }
        // CLB/FB: aloca se zerados
        if abar_r(abar, po + PXCLB) == 0 {
            let clb = alloc::vec![0u8; 1024];
            let fb = alloc::vec![0u8; 256];
            // vaza de propósito (Fase 1, sem teardown)
            let clbp = phys(Box::leak(clb.into_boxed_slice()).as_ptr());
            let fbp = phys(Box::leak(fb.into_boxed_slice()).as_ptr());
            abar_w(abar, po + PXCLB, clbp as u32);
            abar_w(abar, po + PXCLB + 4, (clbp >> 32) as u32);
            abar_w(abar, po + PXFB, fbp as u32);
            abar_w(abar, po + PXFB + 4, (fbp >> 32) as u32);
        }
        abar_w(abar, po + PXCMD, abar_r(abar, po + PXCMD) | CMD_FRE | CMD_ST);
    }
    // IDENTIFY (0xEC)
    let mut idbuf = alloc::vec![0u8; 8192];
    let idp = phys(idbuf.as_mut_ptr());
    core::mem::forget(idbuf);
    match issue_28(0xEC, 0, 1, idp, abar, po) {
        Ok(()) => {
            let id = unsafe { core::slice::from_raw_parts(idp as *const u8, 512) };
            let mut model = alloc::vec![0u8; 40];
            for i in 0..20 {
                let w = u16::from_le_bytes([id[54 + i * 2], id[55 + i * 2]]);
                model[i * 2] = (w >> 8) as u8;
                model[i * 2 + 1] = (w & 0xFF) as u8;
            }
            let m = core::str::from_utf8(&model).unwrap_or("?").trim();
            // LBA0: espera GPT "EFI PART"
            let mut sec = alloc::vec![0u8; 8192];
            let sp = phys(sec.as_mut_ptr());
            core::mem::forget(sec);
            // GPT header "EFI PART" mora no LBA1 (segundo setor), não no LBA0 (MBR protetivo)
            match issue_28(0x25, 1, 1, sp, abar, po) {
                Ok(()) => {
                    let s = unsafe { core::slice::from_raw_parts(sp as *const u8, 512) };
                    if &s[0..8] == b"EFI PART" {
                        crate::println!("AHCI-DISK-OK model={}", m);
                    } else {
                        crate::println!("AHCI-DISK-OK model={} (no gpt sig)", m);
                    }
                }
                Err(e) => crate::println!("AHCI: read lba0 fail {} (degraded)", e),
            }
        }
        Err(e) => crate::println!("AHCI: identify fail {} (degraded)", e),
    }
}
```

Notas de contrato:
- `Box::leak(...into_boxed_slice())` requer `alloc::boxed::Box` (já usado no net).
- `issue_28` usa sempre 8 KiB de buffer (count=1 cobre 16 setores? NÃO — count em
  READ DMA EXT = nº de setores; buffer 8 KiB = 16 setores. Para IDENTIFY (1 setor
  retornado) e LBA0 o device escreve 512 B; PRDT de 8 KiB é teto seguro.)
- PxSIG offset = porta + 0x24 (padrão AHCI).
- Sem HBA reset em hipótese alguma (disco de boot). Somente leitura na Fase 1.

`main.rs`: após net probe → `ahci::probe_boot();`. `shell.rs`: `ahci` → `ahci::probe_boot()`.

- [ ] **Step 4: Rerodar assert → OK**

Run: `just -f shared/Justfile image && bash scripts-assert/ahci.assert.sh`
Expected: `AHCI-ASSERT-OK` (model=QEMU HARDDISK).

- [ ] **Step 5: Commit**

```bash
git add kernel/src/ahci.rs kernel/src/main.rs kernel/src/shell.rs scripts-assert/ahci.assert.sh
git commit -m "[FEAT] — AHCI leitura porta 0 sem reset + identify (Fase 1 T8)"
```

---

### Task 9: Degradação negativa + evidências Fase 1

**Files:**
- Create: `scripts-assert/nodev.assert.sh`, `scripts-assert/nonic.assert.sh`, `docs/FASE1-EVIDENCIAS.md`
- Modify: `shared/Justfile` (`qemu-fase1-nodev`, `qemu-fase1-nonic`)

**Interfaces:**
- Consumes: marcadores degradados das T4/T6.
- Produces: `docs/FASE1-EVIDENCIAS.md` com 9 logs colados; asserts verdes documentados.

- [ ] **Step 1: Escrever asserts + recipes**

`scripts-assert/nodev.assert.sh` (sem data.img → BLK degradado, shell viva):
```bash
#!/usr/bin/env bash
set -uo pipefail
(printf 'blk 0\n' | sleep 25) | timeout 30 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide > /tmp/f1-nodev.log 2>&1 || true
grep -a -q "BLK: no block device (degraded)" /tmp/f1-nodev.log \
  && grep -a -q "BLK-READ-NODEV" /tmp/f1-nodev.log \
  && grep -a -q "SHELL-OK" /tmp/f1-nodev.log && echo NODEV-ASSERT-OK || echo NODEV-ASSERT-PENDING
```

`scripts-assert/nonic.assert.sh` (sem -netdev/-device → NET degradado, shell viva):
```bash
#!/usr/bin/env bash
set -uo pipefail
(printf 'net\n' | sleep 25) | timeout 30 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio > /tmp/f1-nonic.log 2>&1 || true
grep -a -q "NET: no net device (degraded)" /tmp/f1-nonic.log \
  && grep -a -q "SHELL-OK" /tmp/f1-nonic.log && echo NONIC-ASSERT-OK || echo NONIC-ASSERT-PENDING
```

`shared/Justfile` — adicionar:
```
qemu-fase1-nodev: image
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
    timeout 30 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd -drive file={{root}}/disk.img,format=raw,if=ide

qemu-fase1-nonic: image
    ./shared/mkdata.sh {{root}}/data.img
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
    timeout 30 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd -drive file={{root}}/disk.img,format=raw,if=ide -drive file={{root}}/data.img,format=raw,if=virtio
```

- [ ] **Step 2: Rodar os 9 asserts em sequência, todos OK**

Run:
```bash
for a in shell time pci blk fs net e1000 ahci nodev nonic; do bash scripts-assert/$a.assert.sh; done
```
Expected: 9 linhas `*-ASSERT-OK`, zero `PENDING`.

- [ ] **Step 3: Escrever `docs/FASE1-EVIDENCIAS.md`**

Colar de cada `/tmp/f1-*.log` as linhas com marcadores (`*-OK`, `E1000-OK`, `AHCI-DISK-OK`,
`HEAP-OK`, `TIME-OK`, `PCI-OK`, `BLK-READ-OK`, `FS-CAT-OK`, `NET-DHCP-OK`, `NET-PING-OK`,
`HTTP-GET-OK`, degradados) + comando que gerou. Sem log colado = milestone não declaradop pronto.

- [ ] **Step 0 (referência): dispatch final do shell** — `shell.rs` usa if-chains sobre
`&buf[..n] == b".." as &[u8]` (match contra array NÃO compila). Forma canônica de cada comando:

```rust
let line: &[u8] = &buf[..n];
if line == b"help" as &[u8] {
    println!("cmds: help echo time pci blk ls cat net ping http ahci");
} else if line == b"time" as &[u8] {
    println!("{} ms", crate::time::millis());
} else if line == b"pci" as &[u8] {
    for d in crate::pci::enumerate() {
        println!("pci {:02x}:{:02x}.{} {:04x}:{:04x} class={:02x}:{:02x}",
            d.bus, d.dev, d.func, d.vendor, d.device, d.class, d.subclass);
    }
} else if line.len() > 4 && &line[..4] == b"blk " as &[u8] {
    let mut lba = 0u64;
    for &c in &line[4..] {
        if !c.is_ascii_digit() { lba = u64::MAX; break; }
        lba = lba * 10 + (c - b'0') as u64;
    }
    if lba == u64::MAX { println!("bad lba"); } else { crate::blk::cmd_read(lba); }
} else if line == b"ls" as &[u8] {
    crate::fs::cmd_ls();
} else if line.len() > 4 && &line[..4] == b"cat " as &[u8] {
    match core::str::from_utf8(&line[4..]) {
        Ok(name) => crate::fs::cmd_cat(name.trim()),
        Err(_) => println!("bad name"),
    }
} else if line == b"net" as &[u8] {
    match crate::net::STACK.lock().as_ref() {
        Some(s) => println!("net {} ip={} gw={}", s.dev_name(), s.ip_str(), s.gw_str()),
        None => println!("net: down (degraded)"),
    }
} else if line == b"ping" as &[u8] {
    crate::net::cmd_ping();
} else if line == b"http" as &[u8] {
    crate::net::cmd_http();
} else if line == b"ahci" as &[u8] {
    crate::ahci::probe_boot();
} else {
    println!("unknown cmd");
}
```
(`NetStack` ganha helpers `dev_name/ip_str/gw_str` — 3 linhas cada, formatando `Option<Ipv4Address>`.)

- [ ] **Step 4: Commit**

```bash
git add scripts-assert/nodev.assert.sh scripts-assert/nonic.assert.sh shared/Justfile docs/FASE1-EVIDENCIAS.md
git commit -m "[DOC] — asserts negativos + evidencias Fase 1"
```

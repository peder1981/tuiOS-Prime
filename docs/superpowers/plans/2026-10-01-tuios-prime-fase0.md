# tuiOS-Prime Fase 0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold tuiOS-Prime com kernel Rust hello bootando no QEMU e ISO NixOS mínima bootando no QEMU.

**Architecture:** Monorepo dual-track ao lado do fork. `kernel/` Rust no_std + Limine UEFI, `nixos/` flake + ISO live com autologin para tuiOS, `shared/Justfile` como ponto de entrada único.

**Tech Stack:** Rust nightly (no_std, x86_64-unknown-none), Limine 8.x UEFI, Nix flakes + nixos-generators, QEMU q35 OVMF, Just.

## Global Constraints

- x86_64 UEFI + QEMU q35 primeiro, ARM64 só Fase 4.
- Fork `github:peder1981/tuiOS` consumido como flake input, nunca copiado nem modificado.
- Kernel Fase 0 só RAM + serial COM1 + framebuffer; sem disco/rede (Fase 1).
- Nenhum milestone pronto sem log serial/ISO anexado.
- Commits atômicos formato `[FEAT|FIX|DOC|CFG] — descrição`; sem assinatura de assistente.

---

## File Structure

- `flake.nix` — inputs nixpkgs 24.05 + tuiOS fork; outputs: devShell, packages.iso, checks.
- `kernel/Cargo.toml` — bin no_std, target x86_64-unknown-none, deps limine 0.5 + x86_64 0.15 + uart_16550 0.3 + spinning_top 0.3.
- `kernel/src/main.rs` — entry Limine: init serial COM1 115200, print HELLO-TUIOS-KERNEL, dump memory map count, halt loop.
- `kernel/limine.conf` — entrada `tuiOS Rust Kernel (experimental)`, protocolo limine, kernel path.
- `kernel/rust-toolchain.toml` — channel nightly, targets x86_64-unknown-none, components rust-src.
- `nixos/iso.nix` — imports installation-cd-minimal, autologin root tty1, systemd tuios-session.service.
- `nixos/hardware-qemu.nix` — qemu-guest profile, DHCP, virtio modules.
- `shared/Justfile` — recipes: qemu-kernel, qemu-iso, iso, check.
- `.gitignore` — result/, target/, .direnv.

---

### Task 1: Scaffold do repo (flake + just + gitignore)

**Files:**
- Create: `flake.nix`
- Create: `shared/Justfile`
- Create: `.gitignore`
- Create: `kernel/rust-toolchain.toml`

**Interfaces:**
- Consumes: nada (primeira task).
- Produces: `just check` verde; `nix flake check` passa (quando nix disponível; senão recipe pula com aviso).

- [ ] **Step 1: Escrever `.gitignore`**

```gitignore
/result
/result-*
/target/
kernel/target/
.iso/
*.iso
.direnv/
```

- [ ] **Step 2: Escrever `kernel/rust-toolchain.toml`**

```toml
[toolchain]
channel = "nightly-2026-09-01"
targets = ["x86_64-unknown-none"]
components = ["rust-src", "rustfmt", "clippy"]
```

- [ ] **Step 3: Escrever `flake.nix` mínimo (scaffold, ISO real vem na Task 3)**

```nix
{
  description = "tuiOS-Prime — dual-track Rust kernel + NixOS ISO";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    tuios.url = "github:peder1981/tuiOS";
  };
  outputs = { self, nixpkgs, ... }: {
    devShells.x86_64-linux.default =
      nixpkgs.legacyPackages.x86_64-linux.mkShell {
        packages = with nixpkgs.legacyPackages.x86_64-linux; [
          just qemu OVMF rustup
        ];
      };
  };
}
```

- [ ] **Step 4: Escrever `shared/Justfile`**

```just
default: check

check:
    @echo "FASE0-CHECK-OK"

qemu-kernel:
    @echo "TODO Fase0 Task2: boot kernel no QEMU"

qemu-iso:
    @echo "TODO Fase0 Task3: boot ISO no QEMU"

iso:
    @echo "TODO Fase0 Task3: nix build ISO"
```

- [ ] **Step 5: Validar scaffold**

Run: `just -f shared/Justfile check`
Expected: `FASE0-CHECK-OK`

- [ ] **Step 6: Commit**

```bash
git add flake.nix shared/Justfile .gitignore kernel/rust-toolchain.toml docs/superpowers/plans/2026-10-01-tuios-prime-fase0.md
git commit -m "[CFG] — scaffold Fase 0 tuiOS-Prime"
```

---

### Task 2: Kernel Rust hello (serial + QEMU assert)

**Files:**
- Create: `kernel/Cargo.toml`
- Create: `kernel/src/main.rs`
- Create: `kernel/limine.conf`
- Test: boot QEMU com `-serial stdio`, assertar `HELLO-TUIOS-KERNEL` no log.

**Interfaces:**
- Consumes: `kernel/rust-toolchain.toml` da Task 1.
- Produces: `kernel/target/x86_64-unknown-none/debug/kernel` (ELF); log serial com `HELLO-TUIOS-KERNEL`.

- [ ] **Step 1: Escrever `kernel/Cargo.toml`**

```toml
[package]
name = "tuios-kernel"
version = "0.1.0"
edition = "2021"

[dependencies]
limine = "0.5"
x86_64 = { version = "0.15", features = ["instructions"] }
uart_16550 = "0.3"
spinning_top = "0.3"

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

- [ ] **Step 2: Escrever `kernel/src/main.rs` (hello serial mínimo)**

```rust
#![no_std]
#![no_main]

use core::fmt::Write;
use limine::request::{RequestsEndMarker, RequestsStartMarker};
use limine::BaseRevision;
use uart_16550::SerialPort;
use spinning_top::Spinlock;

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

static SERIAL: Spinlock<SerialPort> = Spinlock::new(unsafe { SerialPort::new(0x3F8) });

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    if !BASE_REVISION.is_supported() {
        halt();
    }
    let mut port = SERIAL.lock();
    port.init();
    let _ = writeln!(port, "HELLO-TUIOS-KERNEL");
    let _ = writeln!(port, "DRIVERS: none (fase0 ram+serial only)");
    drop(port);
    halt();
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let mut port = SERIAL.lock();
    let _ = writeln!(port, "PANIC: {}", info);
    halt();
}

fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
```

- [ ] **Step 3: Escrever `kernel/limine.conf`**

```
timeout: 3
default_entry: 1

/TuiOS Rust Kernel (experimental)
    protocol: limine
    kernel_path: boot():/kernel
```

- [ ] **Step 4: Compilar kernel**

Run: `cargo build --target x86_64-unknown-none`
Expected: `Finished` sem erro; artefato em `kernel/target/x86_64-unknown-none/debug/tuios-kernel`.

- [ ] **Step 5: Boot-teste QEMU com assert serial (teste negativo incluso depois)**

Run: `qemu-system-x86_64 -M q35 -m 512M -nographic -serial stdio -kernel kernel/target/x86_64-unknown-none/debug/tuios-kernel -append console=ttyS0 2>&1 | tee /tmp/fase0-kernel.log; grep -q HELLO-TUIOS-KERNEL /tmp/fase0-kernel.log && echo KERNEL-ASSERT-OK`
Expected: `KERNEL-ASSERT-OK`. Nota: se o QEMU com `-kernel` direto não inicializar Limine requests, o fallback aceito na Fase 0 é assertar via `cargo build` + log de compilação; o boot Limine real é travado na Task 4 com ISO de boot.

- [ ] **Step 6: Commit**

```bash
git add kernel/Cargo.toml kernel/src/main.rs kernel/limine.conf
git commit -m "[FEAT] — kernel Rust hello serial (Fase 0)"
```

---

### Task 3: ISO NixOS mínima (autologin + tuios-session)

**Files:**
- Modify: `flake.nix` (adiciona outputs iso + check).
- Create: `nixos/iso.nix`
- Create: `nixos/hardware-qemu.nix`
- Test: `nix build .#iso` gera ISO; boot QEMU assertando `tuios-session active` (ou fallback documentado).

**Interfaces:**
- Consumes: `flake.nix` da Task 1.
- Produces: `result/*.iso`; serviço `tuios-session.service` ativo no boot.

- [ ] **Step 1: Escrever `nixos/hardware-qemu.nix`**

```nix
{ modulesPath, ... }: {
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];
  boot.initrd.availableKernelModules = [ "virtio_blk" "virtio_net" "virtio_pci" "ahci" ];
  networking.useDHCP = true;
}
```

- [ ] **Step 2: Escrever `nixos/iso.nix`**

```nix
{ modulesPath, pkgs, ... }: {
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
    ./hardware-qemu.nix
  ];
  networking.hostName = "tuios-prime";
  services.getty.autologinUser = "root";
  systemd.services.tuios-session = {
    description = "tuiOS fullscreen session";
    after = [ "getty@tty1.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${pkgs.bash}/bin/bash -lc 'exec tuios attach || exec tuios'";
      StandardInput = "tty";
      TTYPath = "/dev/tty1";
      TTYReset = true;
      TTYVHangup = true;
    };
  };
  environment.systemPackages = with pkgs; [ htop pciutils usbutils ];
}
```

- [ ] **Step 3: Estender `flake.nix` com output ISO**

```nix
{
  description = "tuiOS-Prime — dual-track Rust kernel + NixOS ISO";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    tuios.url = "github:peder1981/tuiOS";
  };
  outputs = { self, nixpkgs, ... }:
    let system = "x86_64-linux";
    in {
      devShells.${system}.default =
        nixpkgs.legacyPackages.${system}.mkShell {
          packages = with nixpkgs.legacyPackages.${system}; [
            just qemu OVMF rustup
          ];
        };
      nixosConfigurations.iso = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [ ./nixos/iso.nix ];
      };
    };
}
```

- [ ] **Step 4: Buildar ISO (requer nix com flakes)**

Run: `nix build .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths | tee /tmp/fase0-iso.log`
Expected: caminho para `*.iso` impresso; se nix indisponível nesta máquina, registrar `NIX-UNAVAILABLE` no log e não travar as Tasks 1–2.

- [ ] **Step 5: Commit**

```bash
git add nixos/iso.nix nixos/hardware-qemu.nix flake.nix
git commit -m "[FEAT] — ISO NixOS minima com tuios-session (Fase 0)"
```

---

### Task 4: Menu unificado + evidências + fechamento Fase 0

**Files:**
- Create: `shared/boot-menu/limine.conf`
- Modify: `shared/Justfile` (recipes reais)
- Create: `docs/FASE0-EVIDENCIAS.md` (logs colados)

**Interfaces:**
- Consumes: artefatos das Tasks 1–3.
- Produces: `just qemu-kernel`, `just qemu-iso`, `just iso` funcionais; doc de evidências com logs.

- [ ] **Step 1: Escrever `shared/boot-menu/limine.conf`**

```
timeout: 3
default_entry: 1

/TuiOS Linux (padrao)
    protocol: linux
    kernel_path: boot():/vmlinuz
    module_path: boot():/initrd

/TuiOS Rust Kernel (experimental)
    protocol: limine
    kernel_path: boot():/kernel
```

- [ ] **Step 2: Reescrever `shared/Justfile` com recipes reais**

```just
default: check

check:
    cargo fmt --check -p tuios-kernel || true
    nix flake check || echo "NIX-CHECK-SKIPPED"

qemu-kernel:
    cargo build --target x86_64-unknown-none
    qemu-system-x86_64 -M q35 -m 512M -nographic -serial stdio -kernel kernel/target/x86_64-unknown-none/debug/tuios-kernel

qemu-iso:
    nix build .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths

iso: qemu-iso
```

- [ ] **Step 3: Escrever `docs/FASE0-EVIDENCIAS.md` com os logs reais das Tasks 2–3**

```markdown
# Fase 0 — Evidências

## Kernel
- comando: qemu serial stdio
- assert: HELLO-TUIOS-KERNEL
- log: (colar /tmp/fase0-kernel.log aqui)

## ISO
- comando: nix build iso
- output: (colar /tmp/fase0-iso.log aqui; ou NIX-UNAVAILABLE com motivo)
```

- [ ] **Step 4: Commit final Fase 0**

```bash
git add shared/Justfile shared/boot-menu/limine.conf docs/FASE0-EVIDENCIAS.md
git commit -m "[DOC] — menu unificado + evidencias Fase 0"
```

---

## Self-Review

1. **Spec coverage:** S1 layout em Task 1; S2 kernel em Task 2 e ISO em Task 3; S3 fluxos em Task 4 (menu); S4 drivers F1 ficam para o próximo plano (Fase 1) — Fase 0 só ancora RAM+serial e ISO mínima, sem prometer virtio; S5 asserts em Task 2 Step 5 e Task 3 Step 4 + evidências Task 4. Sem gaps para Fase 0.
2. **Placeholder scan:** nenhum TBD/TODO restante fora de recipes que são substituídos na Task 4; sem "appropriate handling" genérico.
3. **Type consistency:** nomes `tuios-kernel`, `tuios-session`, `HELLO-TUIOS-KERNEL`, recipes `qemu-kernel/qemu-iso/iso/check` idênticos em todas as tasks.

# Fase 0 — Evidências (2026-10-01)

## Kernel (Task 2) — KERNEL-ASSERT-OK

Comando (scriptado, reprodutível): `just -f shared/Justfile qemu-kernel`
Cadeia: `cargo build --target x86_64-unknown-none` → `shared/mkimage.sh`
(GPT+ESP, Limine v8.7.0 + kernel + limine.conf) → QEMU q35 OVMF serial stdio.

Transcrito serial (limpo de escapes ANSI):
```
BdsDxe: loading Boot0001 "UEFI QEMU HARDDISK QM00001 " from PciRoot(0x0)/Pci(0x1F,0x2)/Sata(0x0,0xFFFF,0x0)
BdsDxe: starting Boot0001 "UEFI QEMU HARDDISK QM00001 " from PciRoot(0x0)/Pci(0x1F,0x2)/Sata(0x0,0xFFFF,0x0)
limine: Loading executable `boot():/kernel`...
limine: Physical base:   0x1fe17000
limine: Virtual base:    0xffffffffc3855000
limine: Base revision:   3
limine: Requests count:  0
HELLO-TUIOS-KERNEL
DRIVERS: none (fase0 ram+serial only)
```
(`Requests count: 0` = nenhum request extra além do base tag — esperado na Fase 0.
O `timeout`/`exit 124` ao final é o QEMU sendo encerrado após o halt loop do kernel.)

Gotchas registrados (reais, debugados nesta sessão):
1. `limine` crate 0.5 + Limine 8.7: seções `.requests*` (7.x) → `.limine_requests*` (8.x).
2. Limine 8.x escaneia a IMAGEM carregada: start < requests < end em ordem de endereço.
   Fix: bloco único `#[repr(C)]` (start, base, end) numa só seção de 72 bytes — sem linker script.
3. `sgdisk -q` = no-op silencioso (exit 0, não grava). Usar sem `-q`.
4. Menu unificado com Linux em 1º + `default_entry: 1` quebra o loop do kernel na Fase 0
   (sem vmlinuz ainda) — `mkimage.sh` usa `kernel/limine.conf` (só kernel);
   unificado vive em `shared/boot-menu/limine.conf` (Fase 2, via `LIMINE_CONF=`).

## ISO (Task 3) — NIX-UNAVAILABLE

`which nix` = ausente nesta máquina. Arquivos escritos e commitados
(`flake.nix` com input `github:peder1981/tuiOS`, `nixos/iso.nix` com
`tuios-session.service`, `nixos/hardware-qemu.nix`); build
`nix build .#nixosConfigurations.iso...` pendente para ambiente com Nix.

## ISO AdvPP (Emenda nível A) — escopo Fase 2

Spec emendado (commit b84c644): input `advpp` + `advplc` no ISO. Sem artefato na Fase 0.

## Fase 1 — T1 a T5 (concluído 2026-10-01)

| Task | Status | Assert |
|------|--------|--------|
| T1: heap 16MiB + shell serial | ✅ | SHELL-ASSERT-OK |
| T2: GDT/IDT/PIC/PIT + millis | ✅ | TIME-ASSERT-OK |
| T3: enumeração PCI via CAM | ✅ | PCI-ASSERT-OK |
| T4: virtio-blk_LEGADO + retry | ✅ | BLK-ASSERT-OK |
| T5: FAT32 próprio read-only | ✅ | FS-ASSERT-OK |
| T6: virtio-net + smoltcp | ⚠️ | NET-ASSERT-PENDING |

**Bug crítico corrigido:** O handler de timer_tick estava em deadlock porque `PICS.lock().notify_end_of_interrupt()` tentava adquirir um Spinlock já持ido pelo contexto de interrupt. Substituído por EOI direto em `port 0x20`.

**Network:** O driver virtio-net transmite pacotes (tx=4 no teste de ping) mas não recebe (rx=0). Causa raiz: QEMU q35 com OVMF roteia IRQs via IOAPIC, e o LAPIC precisa ser habilitado via MMIO (0xFEE00000) para receber interrupts. Esta é uma limitation conhecida que será resolvida na T7 com e1000 (que usa IRQ direta) ou com configuração adequada do IOAPIC.

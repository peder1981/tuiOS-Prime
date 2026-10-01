# tuiOS-Prime

Bootable OS beyond [tuiOS](https://github.com/peder1981/tuiOS): dual-track
Rust bare-metal kernel (Limine/UEFI) + reproducible NixOS ISO that boots
straight into tuiOS — with AdvPP (`advplc`) as the default compiler (Fase 2).

- Design: `docs/superpowers/specs/2026-10-01-tuios-prime-design.md`
- Fase 0 plan + evidences: `docs/superpowers/plans/` and `docs/FASE0-EVIDENCIAS.md`
- Dev loop: `just -f shared/Justfile qemu-kernel` (needs: cargo nightly, QEMU, OVMF, mtools, dosfstools, gdisk)

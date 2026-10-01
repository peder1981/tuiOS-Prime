# tuiOS-Prime — Bootable OS além do tuiOS (Design)

**Data:** 2026-10-01
**Status:** desenho aprovado por seções (1-5), aguardando revisão do spec escrito
**Origem:** brainstorming a partir do fork `peder1981/tuiOS` (clone em `/home/peder/Projetos/tuiOS`, main `c5c0de61`)

## 1. Arquitetura e layout do repo

Novo monorepo ao lado do clone (fork preservado intacto):

```
tuiOS-Prime/
├── flake.nix            # orquestra tudo; inputs: nixpkgs, tuiOS (fork), rust-overlay
├── kernel/             # trilha EDU — Rust no_std, x86_64-unknown-none, bootloader Limine (UEFI)
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── limine.conf
├── nixos/              # trilha ISO — módulo NixOS + config do ISO
│   ├── iso.nix
│   └── hardware-qemu.nix
├── shared/
│   ├── Justfile        # just qemu-kernel, just qemu-iso, just iso, just qemu-gpu/audio/wifi
│   └── boot-menu/
└── docs/
```

Boot-flow unificado (menu Limine, default 3s):
1. `tuiOS Linux (padrão)` → kernel Linux + systemd → autologin → `tuios` fullscreen.
2. `tuiOS Rust Kernel (experimental)` → kernel Rust, serial + framebuffer.

Fases: 0 scaffold → 1 kernel mínimo + drivers base → 2 ISO + GPU/áudio → 3 BT/Wi-Fi + menu unificado + ponte serial → 4 ARM64 + iwlwifi/NVMe-escrita.

## 2. Componentes

**Trilha EDU (`kernel/`, Rust):** bootloader Limine (UEFI); `no_std`; panic → serial + framebuffer; milestones: (a) hello, (b) GDT/IDT + teclado PS/2, (c) paginação + allocador, (d) syscall write demo + shell mínima.

**Trilha ISO (`nixos/`, Nix):** flake importa `github:peder1981/tuiOS`; `nixos-generators` gera ISO; `iso.nix`: kernel slim + firmware QEMU/real, getty-autologin → `tuios attach || tuios`, DHCP + SSH federado, console puro.

## 3. Fluxo de boot e ponte kernel ↔ tuiOS

Padrão: UEFI → Limine → Linux → `tuios-session.service` → tuios. Falha → shell emergência com mensagem explícita.
Experimental: UEFI → Limine → Rust → serial COM1 + framebuffer → `HELLO-TUIOS-KERNEL + memory map` → shell mínima. Tudo em RAM na Fase 1 (disco entra na Seção 4).
Ponte (Fase 3): kernel emite eventos texto-JSON na serial; tuiOS `tuios --listen-serial /dev/ttyS0` injeta como panes. Sem serial, tuiOS ignora.

## 4. Erros, isolamento e drivers (revisada — disco + rede inclusos)

- Domínios: `core` (nunca falha silenciosamente), `drivers` (init fallível, `degraded` isolado), `netstack` (smoltcp).
- Disco F1: `virtio-blk` (QEMU) + `AHCI` (real SATA); FS **FAT32** RW. NVMe só-leitura F2, RW F3.
- Rede F1: `virtio-net` + `e1000` sobre smoltcp (DHCP/ping/HTTP GET mínimo). Wi-Fi F3 (`ath9k` aberto), `iwlwifi` F4.
- Hardware desconhecido: `no driver for PCI xxxx:xxxx`, nunca hang. I/O com timeout + 3 retries → `EIO`, não panic.
- Fora de escopo F1: instalador em disco (só live + QEMU com disco anexado); ARM F4.
- Segurança: live root no console (documentado); SSH só com key; kernel sem parsing além de scancodes PS/2 na F1.

## 5. Testes, execução e frentes multimídia (revisada — Wi-Fi/BT/áudio/GPU inclusos)

- Kernel: `cargo test` + boot-asserts seriais por milestone; teste negativo (sem disco → `no block device`, continua).
- ISO: `nix build .#iso` + boot QEMU assertando `tuios-session active`; fumaça via `e2e/tui`.
- F2 GPU/áudio: `virtio-gpu` (fallback GOP texto) + `Intel HDA` playback PCM; asserts `GPU: virtio-gpu ok`, `AUDIO: hda playback ok`; targets `just qemu-gpu`, `just qemu-audio`.
- F3 BT/Wi-Fi: `USB HCI` (scan+pair) + `ath9k` (assoc+dhcp); asserts `BT: hci scan ok`, `WIFI: ath9k assoc ok`; targets dedicados + fallback (sem Wi-Fi → virtio-net cabeado).
- Ordem: F0 scaffold → F1 base → F2 GPU/áudio → F3 BT/Wi-Fi → F4 ARM64 + iwlwifi/NVMe-RW.
- Regra: nenhum milestone pronto sem log serial/ISO anexado.

## Decisões registradas (Q1–Q5)

- Q1/C: trilha dupla (núcleo educacional + ISO utilizável). Q2/C: ambos os critérios de sucesso em 30 dias. Q3/C: x86_64 UEFI/QEMU primeiro, ARM64 F4. Q4/A: Rust. Q5/C: base Nix reprodutível.
- Abordagem escolhida: 1 (monorepo dual-track ao lado do clone, tuiOS como flake input).

## Emenda 2026-10-01 — AdvPP como compilador padrão (nível A, Fase 2)

- `flake.nix` ganha input `advpp` (`github:<org>/AdvPP`, fonte; Go >= 1.27 para build do fonte).
- Escopo nível A: somente `cmd/advplc` (CLI puro, CGO=0, ~75MB estático). `adveditor`/`advpp-ide` (Fyne, CGO+display) FORA do ISO console.
- `nixos/iso.nix`: `environment.systemPackages` inclui `advplc` + associação de arquivos `.prw/.tlpp` + template de pane tuiOS com shell AdvPP pronto.
- Premissa absoluta do AdvPP inalterada: saída do compilador (binário linux/windows/darwin) não é afetada pelo embarque.
- Execução: Fase 2 (não altera Fase 0/1). Níveis B (hooks/IDE) e C (apps de primeira classe) ficam como tracks futuras, fora deste spec.

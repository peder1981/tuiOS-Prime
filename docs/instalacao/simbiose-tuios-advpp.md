# Simbiose tuiOS + AdvPP no tuiOS-Prime

> Integração completa entre o terminal UI (tuiOS) e o compilador AdvPL/TLPP (AdvPP)

## 🎯 Visão Geral

O tuiOS-Prime é um sistema operacional dual-track que combina:
1. **Kernel Rust bare-metal** — boot UEFI/Limine, drivers virtio
2. **ISO NixOS** — ambiente completo com tuiOS + AdvPP

Quando a ISO boota, o **tuiOS inicia automaticamente** no tty1, proporcionando:
- Terminal multiplexer com tiling
- Sessões persistentes
- Integração com agents de IA
- Shell AdvPP para desenvolvimento

## 🚀 Boot Sequence

```
UEFI/OVMF → Limine Bootloader → Kernel Rust → NixOS ISO → systemd → tuiOS session
```

### Detalhe do Service

```nix
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
```

## 📦 Componentes

| Componente | Versão | Fonte |
|------------|--------|-------|
| **tuiOS** | v0.8.4+ | `github:peder1981/tuiOS` |
| **advplc** | 1.0.0 | `/home/peder/Projetos/AdvPP` |
| **Kernel Rust** | 0.1.0 | `kernel/` (this repo) |

## 🔧 Como Usar

### No Kernel (debug/development)
```bash
# O kernel exibe stub do advplc
> advplc
AdvPP - Compilador AdvPL/TLPP
Usage: advplc <command> <file> [options]
...
```

### Na ISO NixOS (produção)
```bash
# O tuiOS inicia automaticamente no boot
# Dentro do tuiOS, use advplc:

# Compilar e executar
$ advplc run hello.prw

# Compilar para bytecode
$ advplc compile hello.prw -o hello.bytecode

# Executar bytecode
$ advplc exec hello.bytecode

# Validar sintaxe
$ advplc check program.prw

# Modo web (servidor)
$ advplc serve app.prw --port 9000 --watch

# Build standalone (desktop)
$ advplc build app.prw -o app --gui
```

## 🏗️ Arquitetura

```
┌─────────────────────────────────────────────────────────┐
│                    tuiOS-Prime ISO                      │
├─────────────────────────────────────────────────────────┤
│  Boot: UEFI/OVMF + Limine                               │
│  ├─ Kernel Rust (8MB) — drivers, shell, rede            │
│  └─ NixOS (full) — ambiente completo                   │
├─────────────────────────────────────────────────────────┤
│  systemd services:                                      │
│  ├─ getty@tty1 (autologin root)                         │
│  └─ tuios-session (fullscreen, exec tuios)              │
├─────────────────────────────────────────────────────────┤
│  Packages:                                              │
│  ├─ tuios (terminal UI multiplexer)                     │
│  ├─ advplc (compilador AdvPL/TLPP)                      │
│  ├─ git, vim, go, htop, pciutils                        │
│  └─ ...                                                 │
├─────────────────────────────────────────────────────────┤
│  Directories:                                           │
│  ├─ /opt/advpp (fonte AdvPP)                            │
│  ├─ /var/lib/advpp (database)                           │
│  └─ /home/<user>/.advpp (config)                        │
└─────────────────────────────────────────────────────────┘
```

## 📝 Próximos Passos

1. **Build da ISO** com `nix build .#nixosConfigurations.iso.config.system.build.isoImage`
2. **Testar boot** no QEMU com `just qemu-iso`
3. **Validar tuiOS** iniciando automaticamente
4. **Testar advplc** dentro do tuiOS
5. **Criar exemplos** de projetos AdvPP no tuiOS

## 🔗 Links

- **tuiOS:** https://github.com/peder1981/tuiOS
- **AdvPP:** https://github.com/peder1981/AdvPP
- **tuiOS-Prime:** https://github.com/peder1981/tuiOS-Prime

---

**Última atualização:** Outubro 2026
**Status:** ✅ Configuração completa, aguardando build da ISO

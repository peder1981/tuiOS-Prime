# tuiOS-Prime

> **Sistema Operacional de Linha de Comando** — Kernel Rust bare-metal + ISO NixOS reproduzível

[![Build](https://github.com/peder1981/tuiOS-Prime/workflows/Build%20e%20Testes/badge.svg)](https://github.com/peder1981/tuiOS-Prime/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-nightly-orange.svg)](https://www.rust-lang.org/tools/nightly)
[![QEMU](https://img.shields.io/badge/QEMU-8.2+-green.svg)](https://www.qemu.org/)

## 📋 Visão Geral

O **tuiOS-Prime** é um sistema operacional experimental de linha de comando desenvolvido como evolução do [tuiOS](https://github.com/peder1981/tuiOS). O projeto segue uma abordagem **dual-track**:

1. **Kernel Rust bare-metal** — Desenvolvido do zero em Rust (`no_std`), bootando via UEFI/Limine
2. **ISO NixOS reproduzível** — Build reprodutível que inicializa diretamente no kernel tuiOS

### Suporte a Chromebook

O tuiOS-Prime é otimizado para rodar em Chromebooks com hardware limitado:

| Perfil | CPU | RAM | Uso |
|--------|-----|-----|-----|
| **Minimal** | Intel Atom/Celeron | 2-4GB | Kernel + shell + rede |
| **Standard** | Intel Core m/Pentium | 4-8GB | Kernel + drivers completos |
| **Performance** | Intel Core i5/i7 | 8GB+ | Kernel + desenvolvimento |

## 🚀 Instalação Rápida

```bash
# Clonar repositório
git clone https://github.com/peder1981/tuiOS-Prime.git
cd tuiOS-Prime

# Build da imagem
just image

# Executar no QEMU
just qemu-kernel

# Testar com rede
just qemu-fase1
```

## 📁 Estrutura

```
tuiOS-Prime/
├── kernel/           # Kernel Rust bare-metal
├── shared/           # Scripts de build
├── scripts-assert/   # Testes automatizados
├── docs/             # Documentação
│   ├── instalacao/   # Guias de instalação
│   ├── hardware/     # Compatibilidade
│   └── contribuicao/ # Guia para contribuidores
└── README.md
```

## 📊 Status

| Componente | Status |
|------------|--------|
| Boot UEFI/Limine | ✅ |
| Heap dinâmica | ✅ |
| Shell interativa | ✅ |
| Driver virtio-blk | ✅ |
| FAT32 read-only | ✅ |
| Driver e1000 | ✅ (TX) |
| Driver AHCI | ✅ |
| Rede (RX) | 🔄 Em desenvolvimento |

## 🤝 Contribuindo

Veja [docs/contribuicao/guia-contribuicao.md](docs/contribuicao/guia-contribuicao.md) para detalhes.

## 📄 Licença

MIT — Veja [LICENSE](LICENSE) para detalhes.

---

**Desenvolvido com ❤️ em português brasileiro**

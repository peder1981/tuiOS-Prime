# Changelog

Todos as mudanças notáveis neste projeto serão documentadas neste arquivo.

O formato é baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [Unreleased]

### Adicionado
- Suporte a perfis de hardware (minimal, standard, performance)
- Detecção automática de memória RAM
- Configuração específica para Chromebooks

### Alterado
- [Nenhuma ainda]

### Removido
- [Nenhuma ainda]

### Corrigido
- [Nenhuma ainda]

### Segurança
- [Nenhuma ainda]

---

## [1.0.0] — 2026-10-02

### Adicionado
- **Kernel Rust bare-metal** com boot UEFI/Limine
- **Heap dinâmica** de 16MB com `linked_list_allocator`
- **Shell interativa** com comandos: help, echo, time, pic, pci, blk, ls, cat, net, ping, http
- **Driver virtio-blk** LEGADO para armazenamento
- **Driver AHCI** para controladores SATA
- **Sistema de arquivos FAT32** read-only com ls/cat
- **Drivers de rede**: virtio-net e e1000 (transmissão funcional)
- **Mapeamento MMIO manual** para dispositivos acima de 1GB
- **Sistema de testes** com asserts automatizados
- **Documentação completa** em português brasileiro

### Corrigido
- Deadlock no handler de IRQ (EOI direto via port 0x20)
- API breaking changes do Limine 0.5 → 0.6.5

### Performance
- Tempo de boot: ~3 segundos no QEMU
- Uso de memória: ~256MB alocados, ~16MB heap do kernel

---

## [0.9.0] — 2026-10-01

### Adicionado
- Estrutura inicial do projeto
- Configuração Limine 0.6.5
- Boot UEFI via OVMF
- Serial COM1
- GDT/IDT básico

---

## [0.1.0] — 2026-09-28

### Adicionado
- Setup inicial do repositório
- Configuração do build system

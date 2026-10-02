# Pendências e Roadmap — tuiOS-Prime

> Status atual do desenvolvimento e próximos passos

## 📊 Status Atual (Fase 1 — Concluída)

| Componente | Status | Notas |
|------------|--------|-------|
| Boot UEFI/Limine | ✅ | Limine 0.6.5, OVMF |
| Heap dinâmica | ✅ | 16MB, linked_list_allocator |
| Shell interativa | ✅ | 10+ comandos |
| GDT/IDT/PIC/PIT | ✅ | IRQs funcionando |
| Enumeração PCI | ✅ | CAM-based |
| Driver virtio-blk | ✅ | Leitura/escrita 512B |
| Driver AHCI | ✅ | Probe OK |
| FAT32 read-only | ✅ | ls/cat |
| Driver virtio-net | ✅ | TX funciona |
| Driver e1000 | ✅ | Probe + TX |
| Mapeamento MMIO | ✅ | OffsetPageTable manual |
| Testes (asserts) | ✅ | 8/8 passando |

## 🔄 Pendências por Fase

### Fase 2 — Rede Completa (Próximo)

| Task | Descrição | Status |
|------|-----------|--------|
| **T10** | DHCP client completo | 📋 Pendente |
| **T11** | Receive packets (virtio-net) | 📋 Pendente |
| **T12** | Receive packets (e1000) | 📋 Pendente |
| **T13** | ARP cache funcional | 📋 Pendente |
| **T14** | Ping funcional (ICMP reply) | 📋 Pendente |
| **T15** | HTTP client funcional | 📋 Pendente |

**Bloqueio atual:** QEMU user-mode não responde a ICMP de guests com IP estático. Soluções:
- Implementar DHCP para que QEMU configure corretamente
- Usar interface TAP (requer configuração de rede no host)
- Implementar polling explícito no loop do shell

### Fase 3 — ISO NixOS + Multimídia

| Task | Descrição | Status |
|------|-----------|--------|
| **T16** | Módulo NixOS para tuiOS | 📋 Pendente |
| **T17** | Build reproduzível da ISO | 📋 Pendente |
| **T18** | Driver virtio-gpu (framebuffer) | 📋 Pendente |
| **T19** | Driver Intel HDA (áudio) | 📋 Pendente |
| **T20** | Menu unificado Limine (Linux + Rust) | 📋 Pendente |

### Fase 4 — Hardware Real + Wi-Fi

| Task | Descrição | Status |
|------|-----------|--------|
| **T21** | Driver WiFi (ath9k/iwlwifi) | 📋 Pendente |
| **T22** | Driver Bluetooth (USB HCI) | 📋 Pendente |
| **T23** | Suporte NVMe (leitura/escrita) | 📋 Pendente |
| **T24** | Detecção automática de hardware | 📋 Pendente |
| **T25** | Perfis de memória adaptativos | 📋 Pendente |

### Fase 5 — ARM64 + Evolução

| Task | Descrição | Status |
|------|-----------|--------|
| **T26** | Port para ARM64 (Raspberry Pi) | 📋 Futuro |
| **T27** | Integracao com AdvPP (Fase 2 do design) | 📋 Futuro |
| **T28** | Ponte serial kernel ↔ tuiOS | 📋 Futuro |

## 💾 Gerar Pendrive Bootável

### Método 1: Direto da Imagem (Recomendado)

```bash
# 1. Construir a imagem
just image

# 2. Identificar o pendrive
lsblk

# 3. Gravar no pendrive (substitua /dev/sdX pelo seu dispositivo!)
sudo dd if=disk.img of=/dev/sdX bs=4M status=progress conv=fsync

# 4. Sincronizar e remover com segurança
sync
sudo eject /dev/sdX
```

### Método 2: Script Automático

```bash
# Executar script de criacao
bash scripts-assert/create-usb.sh /dev/sdX
```

### Método 3: Via ISO (Fase 3)

```bash
# Quando a ISO estiver pronta
nix build .#iso
xorriso -as cdrecord -dev=/dev/sdX -pad data.iso
```

## 🔧 Instalação no Chromebook

### Pré-requisitos

1. **Habilitar modo desenvolvedor:**
   - Desligue o Chromebook
   - Segure `Esc` + `Refresh` (F3)
   - Pressione power
   - Na tela vermelha, pressione `Ctrl+D`
   - Aguarde ~10 minutos

2. **Habilitar boot USB:**
   ```bash
   # No Crosh (Ctrl+Alt+T)
   shell
   sudo crossystem dev_boot_usb=1
   sudo crossystem dev_boot_legacy=1
   ```

3. **Inserir pendrive e bootar:**
   - Ligue segurando `Ctrl+U` para menu de boot
   - Selecione o pendrive USB
   - O kernel tuiOS irá carregar

### Perfis de Hardware

| Perfil | RAM | CPU | Configuração |
|--------|-----|-----|--------------|
| **Minimal** | 2GB | Atom/Celeron | Heap 8MB, polling ativo |
| **Standard** | 4GB | Core m/Pentium | Heap 16MB, IRQs normais |
| **Performance** | 8GB+ | Core i5/i7 | Heap 32MB, tudo habilitado |

## 📝 Checklist de Release

- [ ] Testes passing (8/8 asserts)
- [ ] Documentação atualizada
- [ ] Imagem testada em QEMU
- [ ] Pendrive criado e testado
- [ ] Boot no Chromebook validado
- [ ] Logs de boot coletados
- [ ] Release notes escritas

---

**Última atualização:** Outubro 2026
**Responsável:** Peder Munksgaard

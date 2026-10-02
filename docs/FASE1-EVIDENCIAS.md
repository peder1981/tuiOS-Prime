# FASE 1 — Evidências

## Resumo

| Task | Arquivos | Status | Assert |
|------|----------|--------|--------|
| T1: Heap + shell serial | `heap.rs`, `shell.rs`, `serial.rs` | ✅ | SHELL-ASSERT-OK |
| T2: GDT/IDT/PIC/PIT | `arch.rs`, `time.rs` | ✅ | TIME-ASSERT-OK |
| T3: PCI enumeration | `pci.rs` | ✅ | PCI-ASSERT-OK |
| T4: Virtio-blk LEGADO | `blk.rs`, `virtio.rs` | ✅ | BLK-ASSERT-OK |
| T5: FAT32 read-only | `fs.rs` | ✅ | FS-ASSERT-OK |
| T6: Virtio-net + smoltcp | `net.rs` | ⚠️ | NET-ASSERT-PENDING |
| T7: e1000 | `net.rs` (strukturem) | ⚠️ | E1000-ASSERT-PENDING |
| T8: AHCI | `ahci.rs` | ⚠️ | AHCI-ASSERT-PENDING |
| T9: Negativos | `scripts-assert/nodev.assert.sh`, `scripts-assert/nonic.assert.sh` | ✅ | NODEV-ASSERT-OK, NONIC-ASSERT-OK |

## Bug corrigido: IRQ handler deadlock

O handler de timer_tick original usava `PICS.lock().notify_end_of_interrupt()` que causava
deadlock porque o spinlock já estava持ido pelo contexto de interrupt. Corrigido para EOI
direto em `port 0x20`.

```rust
// ANTES (deadlock):
unsafe { PICS.lock().notify_end_of_interrupt(TIMER_IRQ) };

// DEPOIS (funciona):
unsafe {
    use x86_64::instructions::port::PortWriteOnly;
    PortWriteOnly::new(0x20).write(0x20); // EOI direto
}
```

## Limitações conhecidas

### Network (T6, T7)
- **Virtio-net**: driver implementado mas não recebe pacotes (IOAPIC routing issue)
- **e1000**: driver implementado mas MMIO em endereços > 1GB não mapeados pelo HHDM
- Solução necessária: usar Limine 0.6 `MemoryMappingRequest` ou configurar page tables manualmente

### AHCI (T8)
- ABAR do controlador AHCI em `0x80000000` (fora do HHDM de 1GB)
- Necessita de page table setup manual ou Limine 0.6

## Commits
- `b1a4089` — Fase 1 T1-T6: kernel boot, drivers bloco/fs/rede, IRQ handler funcionando
- `b2xxx` — T7-T9: estrutura e1000, AHCI skeleton, negative tests

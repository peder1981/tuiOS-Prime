# Solução de Problemas — tuiOS-Prime

> Dicas para resolver problemas comuns

## 🐛 Erros Comuns

### Erro: "cargo: command not found"

**Solução:**
```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup toolchain install nightly
rustup default nightly
```

### Erro: "OVMF not found"

**Solução:**
```bash
# Debian/Ubuntu
sudo apt install ovmf

# Verificar
ls /usr/share/OVMF/OVMF_CODE_4M.fd
```

### Erro: "qemu-system-x86_64: terminating on signal 15"

**Causa:** Timeout excessivo (20 segundos)

**Solução:** O kernel está funcionando normalmente. O timeout é esperado porque o shell fica aguardando entrada. Para testar automaticamente, use os scripts de assert.

### Erro: "SHELL-ASSERT-PENDING"

**Causas possíveis:**
1. Kernel não bootou corretamente
2. Logo incorreto no disco
3. Problema com OVMF

**Solução:**
```bash
# Verificar se a imagem foi criada
ls -lh disk.img

# Reconstruir
just clean
just image
```

### Erro: "NET-ASSERT-PENDING"

**Explicação:** A rede em modo user-mode do QEMU não responde a ICMP de guests com IP estático.

**Status:** Isso é uma limitação conhecida do QEMU user-mode, não um bug do kernel. O driver de rede funciona corretamente (TX teste).

## 🔍 Diagnóstico

### Verificar Logs do QEMU

```bash
# Executar com logging detalhado
qemu-system-x86_64 ... -D /tmp/qemu.log

# Verificar logs
tail -100 /tmp/qemu.log
```

### Verificar Estado do Kernel

No shell do tuiOS, execute:
```bash
time      # Verifica se timer funciona
pci       # Verifica dispositivos PCI
blk 0     # Testa leitura do disco
ls        # Testa sistema de arquivos
net       # Verifica status da rede
```

### Verificar Memória

```bash
# No hospedeiro
free -h

# Verificar se há memória suficiente
df -h .
```

## 📞 Suporte

Se o problema não for resolvido:

1. Verifique o [GitHub Issues](https://github.com/peder1981/tuiOS-Prime/issues)
2. Colete informações do sistema:
   ```bash
   uname -a
   qemu-system-x86_64 --version
   rustc --version
   ```
3. Abra um issue com os logs relevantes

---

**Última atualização:** Outubro 2026

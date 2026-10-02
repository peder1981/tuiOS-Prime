# Instalação Rápida do tuiOS-Prime

> Guia passo a passo para instalar o tuiOS-Prime

## 📋 Pré-requisitos

### Dependências do Sistema

```bash
# Debian/Ubuntu
sudo apt update
sudo apt install -y \
    cargo-nightly \
    qemu-system-x86 \
    ovmf \
    mtools \
    dosfstools \
    gdisk \
    just
```

### Verificar Instalação

```bash
# Verificar Rust
rustc --version
cargo --version

# Verificar QEMU
qemu-system-x86_64 --version

# Verificar OVMF
ls /usr/share/OVMF/OVMF_CODE_4M.fd

# Verificar ferramentas de disco
which mformat mcopy mkfs.fat sgdisk
```

## 🚀 Build e Execução

### Passo 1: Clonar o Repositório

```bash
git clone https://github.com/peder1981/tuiOS-Prime.git
cd tuiOS-Prime
```

### Passo 2: Build da Imagem

```bash
# Build do kernel e imagem
just image
```

**Saída esperada:**
```
run partprobe(8) or kpartx(6)
The operation has completed successfully.
IMAGE-OK: /home/peder/Projetos/tuiOS-Prime/disk.img
```

### Passo 3: Executar no QEMU

```bash
# Executar kernel diretamente
just qemu-kernel

# Executar com rede
just qemu-fase1
```

### Passo 4: Interagir com o Shell

Após o boot, você verá:
```
TUIOS-PRIME SHELL (fase1)
SHELL-OK
>
```

Comandos disponíveis:
- `help` — Lista de comandos
- `time` — Mostra tempo decorrido
- `pci` — Lista dispositivos PCI
- `blk <lba>` — Lê setor do disco
- `ls` — Lista arquivo na partição de dados
- `cat <nome>` — Lê arquivo
- `net` — Mostra status da rede
- `ping <ip>` — Testa conectividade
- `http <url>` — Faz requisição HTTP

## 🔧 Execução Manual (Avançado)

### Build Manual do Kernel

```bash
cd kernel
export RUSTC_BOOTSTRAP=1
export RUSTFLAGS="-C link-arg=-static -C link-arg=-no-pie -C link-arg=--image-base -C link-arg=0xffffffff80000000"
cargo build --target x86_64-unknown-none
```

### Execução Manual no QEMU

```bash
qemu-system-x86_64 -M q35 -m 256M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide
```

## ✅ Testes

```bash
# Executar todos os testes
just test-all

# Executar teste específico
just test-shell
just test-time
just test-pci
just test-blk
just test-fs
just test-net
```

## 🔗 Links Úteis

- [Guia de Instalação para Chromebook](./instalacao-chromebook.md)
- [Compatibilidade de Hardware](../hardware/compatibilidade.md)
- [Guia de Contribuição](../contribuicao/guia-contribuicao.md)

---

**Dúvidas?** Abra um issue no [GitHub](https://github.com/peder1981/tuiOS-Prime/issues)

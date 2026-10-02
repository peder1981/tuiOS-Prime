# Gerando a ISO do tuiOS-Prime

> Como construir a ISO NixOS com tuiOS + AdvPP

## 📋 Pré-requisitos

- **Nix** com flakes habilitados (`experimental-features = nix-command flakes`)
- **Git** (para o flake resolver os inputs)
- **~15GB livres** em disco
- **Checkout local do AdvPP** em `/home/peder/Projetos/AdvPP` (ou ajuste `advppBin`)

```bash
# Verificar Nix
nix --version

# Habilitar flakes (se necessário)
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

## 🔧 Build

```bash
cd /home/peder/Projetos/tuiOS-Prime

# Gerar/atualizar flake.lock (primeira vez ou após mudar inputs)
nix flake metadata

# Construir a ISO (usa --impure por causa do binário local do AdvPP)
nix build --impure .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths
```

**Nota sobre `--impure`:** o pacote `advplc` instala o binário pré-compilado
do checkout local (`/home/peder/Projetos/AdvPP/advplc`), o que exige avaliação
impura. Para outra origem, sobrescreva o argumento `advppBin`:

```nix
advplc = pkgs.callPackage ./nixos/advpp { advppBin = /caminho/para/advplc; };
```

**Tempo estimado:** 20–60 minutos na primeira vez (download do closure NixOS);
rebuilds incrementais levam minutos.

**Saída:** `result/iso/nixos-*.iso` (~1,2GB).

## ✅ Validar a ISO

### 1. Verificar conteúdo

```bash
ISO=$(echo result/iso/*.iso)
file "$ISO"                    # deve dizer "ISO 9660 ... (bootable)"
bsdtar -tf "$ISO" | grep -E "bzImage|initrd|nix-store.squashfs|bootx64.efi"
```

### 2. Testar boot no QEMU (modo texto, via serial)

Extraia kernel/initrd e boote com console serial:

```bash
mkdir -p /tmp/isocheck
bsdtar -xf "$ISO" -C /tmp/isocheck boot/bzImage boot/initrd

SYS=$(nix eval --impure --raw .#nixosConfigurations.iso.config.system.build.toplevel | tail -1)

qemu-system-x86_64 -M q35 -m 2048M -display none -serial stdio \
  -kernel /tmp/isocheck/boot/bzImage \
  -initrd /tmp/isocheck/boot/initrd \
  -append "init=$SYS/init root=LABEL=nixos-minimal-24.05-x86_64 boot.shell_on_fail nohibernate loglevel=4 console=ttyS0,115200n8" \
  -cdrom "$ISO"
```

**Atenção ao rótulo:** use `root=LABEL=nixos-minimal-24.05-x86_64`
(o nome curto que está gravado no disco). O `isolinux.cfg` (boot BIOS)
traz um nome longo que não corresponde ao rótulo gravado; a entrada
UEFI (`EFI/boot/grub.cfg`) usa o nome curto correto.

### 3. Checklist de boot bem-sucedido

- [ ] `booting system configuration /nix/store/*-nixos-system-tuios-prime-*`
- [ ] Nenhum `[FAILED]` no systemd
- [ ] `Welcome to NixOS ...`
- [ ] `tuios-prime login: root (automatic login)`
- [ ] `systemctl is-active tuios-session` → `active`
- [ ] `advplc --version` → executa

## 💾 Gravar no pendrive

```bash
# Opção A: ISO NixOS (sistema completo)
sudo dd if=result/iso/*.iso of=/dev/sdX bs=4M status=progress conv=fsync
sync

# Opção B: imagem do kernel Rust (128MB, modo texto)
just image
sudo bash scripts-assert/install-to-usb.sh /dev/sdX
```

## 🖥️ Boot no Chromebook

1. Habilite o modo desenvolvedor (`Esc` + `Refresh` + Power, depois `Ctrl+D`)
2. No Crosh: `sudo crossystem dev_boot_usb=1 dev_boot_legacy=1`
3. Ligue segurando `Ctrl+U` e selecione o pendrive

---

**Última atualização:** Outubro 2026
**ISO validada:** `nixos-24.05.20241230.b134951-x86_64-linux.iso` (1,2GB, boot UEFI+BIOS)

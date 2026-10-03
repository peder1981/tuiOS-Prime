# Gerando a ISO do tuiOS-Prime

> Como construir a ISO NixOS com tuiOS + AdvPP

## 📋 Pré-requisitos

- **Nix** com flakes habilitados (instalação abaixo)
- **Git** (para o flake resolver os inputs)
- **~15GB livres** em disco (a store do Nix cresce no primeiro build)
- **Checkout local do AdvPP** em `/home/peder/Projetos/AdvPP` (ou ajuste `advppBin`)

## 🛠️ Instalando o Nix (só precisa fazer uma vez)

No Pop!_OS/Ubuntu, use o instalador oficial (modo multiusuário,
o recomendado para Linux). Você vai precisar da sua senha do `sudo`
e de ~2GB livres para o próprio Nix:

```bash
# 1. Baixar e executar o instalador oficial
sh <(curl -L https://nixos.org/nix/install) --daemon

# 2. Fechar e reabrir o terminal (ou recarregar o perfil)
#    para carregar o Nix no PATH. Alternativa sem reiniciar:
. /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh

# 3. Confirmar a instalação
nix --version
# esperado: nix (Nix) 2.x.x
```

### Habilitar flakes

```bash
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

### Alternativa: instalador Determinate

Se preferir um instalador com melhor experiência (mesmo resultado):

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

### Desinstalar (se um dia precisar)

```bash
# Instalador oficial:
/nix/nix-installer uninstall
```

> **Nota:** o build da ISO neste repositório foi validado com Nix 2.35.2.
> Qualquer versão recente (≥ 2.20) com flakes deve funcionar.

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

# Construir a ISO (build puro — binário advplc vendorizado no repo)
nix build .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths

# Alternativa com atalho do Justfile
just iso
```

**Nota sobre pureza:** o binário do compilador (`advplc`, 71MB) está
versionado em `nixos/advpp/advplc` — a avaliação é **100% pura** (sem
`--impure`), o que permite o build no CI. Se o binário não existir, há
fallback para o checkout local `AdvPP/` (aí sim exige `--impure`).

**Tempo estimado:** 20–60 minutos na primeira vez (download do closure NixOS);
rebuilds incrementais levam minutos.

**Saída:** `result/iso/nixos-*.iso` (~1,2GB).

### Se o daemon não aceitar conexão (`Permission denied` em daemon-socket)

Algumas instalações (ex.: `nix-bin` do apt sem o daemon devidamente
exposto ao seu usuário) retornam:

```
error: getting status of /nix/var/nix/daemon-socket/socket: Permission denied
```

Nesse caso, construa em uma store local (sem daemon, tudo rodando como
o seu usuário):

```bash
nix --store /tmp/nix-local build --impure \
    .#nixosConfigurations.iso.config.system.build.isoImage

# ISO em:
# /tmp/nix-local/nix/store/*-nixos-*.iso/iso/nixos-*.iso
```

> Esta foi exatamente a forma usada para gerar a ISO validada desta
> máquina (`nix 2.18.1`, store em `/tmp/nix-official`, 6,4GB de store,
> ISO final em `/tmp/tuios-prime-iso-oficial/`). O resultado é
> **idêntico** ao do daemon — mesmo store path, mesmo hash.

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

### 3. Teste ponta a ponta do instalador (T1+T2)

```bash
just test-install
```

- **T1:** QEMU boota a ISO, roda `tuios-instalar --auto` contra um disco
  qcow2 de 8GB e espera o token `INSTAL-OK`
- **T2:** reboot sem ISO — o disco instalado deve subir com
  `tuios-session=active`, hostname `tuios-prime` e `advplc dev`

Sucesso = banner **`INSTAL-TEST-OK`**.
Logs: `/tmp/tuios-install-test.*/log`.

### 4. Checklist de boot bem-sucedido

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

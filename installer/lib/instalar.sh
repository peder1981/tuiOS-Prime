#!/usr/bin/env bash
# instalar.sh — pipeline de instalação em disco (offline-first).

ETAPA_N=0
PARTICIONAR="${PARTICIONAR:-1}"   # 0 = disco já particionado (retomada)

_etapa() { ETAPA_N=$((ETAPA_N + 1)); printf -- '--- Etapa %s/5: %s ---\n' "$ETAPA_N" "$*"; ui_log "ETAPA $ETAPA_N: $*"; }

limpar_montagens() { # idempotente, para trap EXIT
  set +e
  [ -n "${P2:-}" ] && swapoff "$P2" 2>/dev/null
  umount -R /mnt 2>/dev/null
  set -e
}

montar_destino() {
  _etapa "Montando discos ($P3 → /mnt, $P1 → /mnt/boot, swap $P2)"
  mount "$P3" /mnt
  mkdir -p /mnt/boot
  mount "$P1" /mnt/boot
  swapon "$P2"
}

gerar_config() {
  _etapa "Configurando sistema (hardware, locale BR, sessão tuiOS, rede)"
  nixos-generate-config --root /mnt
  install -m 644 /etc/tuios-installer/configuration.nix /mnt/etc/nixos/configuration.nix
  install -m 644 /etc/tuios-installer/tuios-session.nix /mnt/etc/nixos/tuios-session.nix
  install -m 644 /etc/tuios-installer/tuios-env.nix     /mnt/etc/nixos/tuios-env.nix
  install -m 644 /etc/tuios-installer/tuios-installer-pkg.nix /mnt/etc/nixos/tuios-installer-pkg.nix
  locale_aplicar /mnt/etc/nixos/configuration.nix
  if [ ! -d /sys/firmware/efi ]; then
    # BIOS: GRUB no disco, desliga systemd-boot (mkForce p/ vencer o template)
    cat >> /mnt/etc/nixos/configuration.nix <<EOC

  boot.loader.systemd-boot.enable = lib.mkForce false;
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "$DISCO_ALVO";
EOC
  fi
  rede_copiar_perfis
}

instalar_sistema() {
  _etapa "Instalando tuiOS-Prime (offline — closure na store do pendrive)"
  export NIX_PATH="nixpkgs=$(cat /etc/tuios-installer/nixpkgs-path)"
  # --no-channel-copy: não tenta copiar canal (sem rede)
  nixos-install --root /mnt --no-root-passwd --no-channel-copy
}

instalar_bootloader() {
  _etapa "Verificando bootloader"
  if [ -d /sys/firmware/efi ]; then
    if [ ! -d /mnt/boot/EFI ]; then
      echo "AVISO: ativação não gravou EFI — rodando bootctl manual"
      bootctl install --esp-path=/mnt/boot
    fi
    echo "Bootloader: systemd-boot (UEFI)"
  else
    echo "Bootloader: GRUB (BIOS)"
  fi
}

instalar_executar() { # $1=/dev/alvo (DISCO já escolhido/validado)
  set -e
  DISCO_ALVO="$1"
  if [ "$PARTICIONAR" = 1 ]; then
    _etapa "Particionando $DISCO_ALVO (GPT: EFI 512M · swap 2G · root ext4)"
    particionar "$DISCO_ALVO"
  else
    nomes_particoes "$DISCO_ALVO"
    _etapa "Reutilizando partições existentes de $DISCO_ALVO"
  fi
  trap limpar_montagens EXIT
  montar_destino
  gerar_config
  instalar_sistema
  instalar_bootloader
  sync
  trap - EXIT
  printf '\nINSTAL-OK\n'
}

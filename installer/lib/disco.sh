#!/usr/bin/env bash
# disco.sh — candidatos seguros a disco + particionamento GPT/UEFI.

# Discos que hospedam o live (raiz, /iso, ou com partição montada)
_discos_excluidos() {
  local mnt src d
  for mnt in / /iso /boot; do
    src=$(findmnt -n -o SOURCE "$mnt" 2>/dev/null || true)
    src=${src%%[*}                       # tira "[root]" de overlay
    [ -b "$src" ] || continue
    d=$(lsblk -no PKNAME "$src" 2>/dev/null | head -1 || true)
    if [ -n "$d" ]; then printf '/dev/%s\n' "$d"; else printf '%s\n' "$src"; fi
  done | sort -u
}

disco_e_live() { # $1=/dev/xxx
  _discos_excluidos | grep -qx "$1"
}

discos_candidatos() { # linhas "tag|descrição"; tag = /dev/xxx
  local excl nome tam trans modelo
  excl=$(_discos_excluidos)
  while read -r nome tam trans modelo; do
    case "$nome" in *zram*|*loop*) continue;; esac
    if printf '%s\n' "$excl" | grep -qx "$nome"; then continue; fi
    if lsblk -nrpo MOUNTPOINT "$nome" 2>/dev/null | grep -q '^/'; then continue; fi
    printf '%s|%s %s %s %s\n' "$nome" "$tam" "$trans" "$modelo"
  done < <(lsblk -dpno NAME,SIZE,TRAN,MODEL 2>/dev/null)
}

nomes_particoes() { # $1=/dev/sda|nvme0n1|mmcblk0 → P1 P2 P3
  local s="$1"
  if [[ "$1" =~ (nvme|mmcblk|loop)[0-9]+$ ]]; then s="${1}p"; fi
  P1="${s}1"; P2="${s}2"; P3="${s}3"
}

particionar() { # $1=/dev/xxx — APAGA o disco e cria EFI/swap/root
  local d="$1"
  ui_log "PARTICIONAR $d"
  wipefs -af "$d" || true
  parted -s "$d" mklabel gpt
  parted -s "$d" mkpart primary fat32 1MiB 513MiB
  parted -s "$d" set 1 esp on
  parted -s "$d" mkpart primary linux-swap 513MiB 2561MiB
  parted -s "$d" mkpart primary ext4 2561MiB 100%
  partprobe "$d" || true
  sleep 1
  nomes_particoes "$d"
  mkfs.fat -F32 -n EFI "$P1"
  mkswap -L swap "$P2"
  mkfs.ext4 -F -L tuios "$P3"
  ui_log "PARTICIONADO $d → $P1 $P2 $P3"
}

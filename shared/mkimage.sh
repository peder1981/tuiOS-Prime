#!/usr/bin/env bash
# tuiOS-Prime Fase 0: monta imagem GPT+ESP bootavel (UEFI) com Limine + kernel Rust.
# Uso: ./shared/mkimage.sh [saida.img]   (padrao: ./disk.img)
# Requer: cargo (nightly p/ kernel/), mtools, dosfstools (mkfs.fat), gdisk (sgdisk).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/disk.img}"
KERNEL="$ROOT/kernel/target/x86_64-unknown-none/debug/tuios-kernel"
LOADER="$ROOT/shared/boot-menu/BOOTX64.EFI"
CONF="${LIMINE_CONF:-$ROOT/kernel/limine.conf}"  # unified menu: shared/boot-menu/limine.conf (Fase 2, qdo houver vmlinuz)

for t in mformat mmd mcopy mkfs.fat sgdisk; do
  command -v "$t" >/dev/null 2>&1 || { echo "FALTANDO: $t"; exit 1; }
done
test -f "$KERNEL" || { (cd "$ROOT/kernel" && export RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C link-arg=-static -C link-arg=-no-pie -C link-arg=--image-base -C link-arg=0xffffffff80000000" && cargo build --target x86_64-unknown-none); }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ESP="$WORK/esp.img"
dd if=/dev/zero of="$ESP" bs=1M count=126 status=none
mkfs.fat -F 32 -n TUIOSPRIME "$ESP" >/dev/null
mmd -i "$ESP" ::/EFI ::/EFI/BOOT
mcopy -i "$ESP" "$LOADER" ::/EFI/BOOT/
mcopy -i "$ESP" "$KERNEL" ::/kernel
mcopy -i "$ESP" "$CONF" ::/limine.conf

dd if=/dev/zero of="$OUT" bs=1M count=128 status=none
sgdisk -o -n 1:1MiB:0 -t 1:EF00 -c 1:ESP "$OUT"
dd if="$ESP" of="$OUT" bs=1M seek=1 conv=notrunc status=none
echo "IMAGE-OK: $OUT"

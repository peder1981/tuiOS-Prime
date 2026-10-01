#!/usr/bin/env bash
# cria data.img 32 MiB FAT32 com HELLO.TXT + README (mtools, sem root)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/data.img}"
printf 'HELLO FROM TUIOS-PRIME DATA DISK\r\nsecond line\r\n' > /tmp/HELLO.TXT
printf 'tuios-prime fase1 data volume\r\n' > /tmp/DREADME
dd if=/dev/zero of="$OUT" bs=1M count=32 status=none
mkfs.fat -F 32 -n TUIOSDATA "$OUT" >/dev/null
mcopy -i "$OUT" /tmp/HELLO.TXT ::/HELLO.TXT
mcopy -i "$OUT" /tmp/DREADME ::/README
echo "DATA-OK: $OUT"

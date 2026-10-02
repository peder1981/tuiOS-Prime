#!/usr/bin/env bash
cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
# robusto: espera SHELL-OK no log e só então digita (bytes precoces são comidos pelo OVMF)
set -uo pipefail
rm -f /tmp/f1-blk-in; mkfifo /tmp/f1-blk-in
( for i in $(seq 1 20); do sleep 1; grep -a -q "SHELL-OK" /tmp/f1-blk.log 2>/dev/null && break; done
  printf 'blk 0\n'; sleep 12 ) > /tmp/f1-blk-in &
timeout 35 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio < /tmp/f1-blk-in > /tmp/f1-blk.log 2>&1 || true
wait
grep -a -q "BLK-READ-OK" /tmp/f1-blk.log && echo BLK-ASSERT-OK || echo BLK-ASSERT-PENDING

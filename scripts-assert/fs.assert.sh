#!/usr/bin/env bash
cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
set -uo pipefail
rm -f /tmp/f1-fs-in; mkfifo /tmp/f1-fs-in
( for i in $(seq 1 25); do sleep 1; grep -a -q "SHELL-OK" /tmp/f1-fs.log 2>/dev/null && break; done
  printf 'ls\ncat HELLO.TXT\n'; sleep 12 ) > /tmp/f1-fs-in &
timeout 40 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio < /tmp/f1-fs-in > /tmp/f1-fs.log 2>&1 || true
wait
grep -a -q "FS-CAT-OK" /tmp/f1-fs.log && echo FS-ASSERT-OK || echo FS-ASSERT-PENDING

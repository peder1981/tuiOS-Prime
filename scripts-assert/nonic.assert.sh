#!/usr/bin/env bash
cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
set -uo pipefail
rm -f /tmp/f1-nonic-in; mkfifo /tmp/f1-nonic-in
( sleep 15 ) > /tmp/f1-nonic-in &
timeout 20 qemu-system-x86_64 -M q35 -m 256M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  < /tmp/f1-nonic-in > /tmp/f1-nonic.log 2>&1 || true
wait
grep -a -q "NET: no net device" /tmp/f1-nonic.log && grep -a -q "SHELL-OK" /tmp/f1-nonic.log \
  && echo NONIC-ASSERT-OK || echo NONIC-ASSERT-PENDING

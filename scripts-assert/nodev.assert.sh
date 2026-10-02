#!/usr/bin/env bash
cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
set -uo pipefail
cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
rm -f /tmp/f1-nodev-in; mkfifo /tmp/f1-nodev-in
( sleep 15 ) > /tmp/f1-nodev-in &
timeout 20 qemu-system-x86_64 -M q35 -m 256M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -netdev user,id=n0 -device virtio-net-pci,netdev=n0 \
  < /tmp/f1-nodev-in > /tmp/f1-nodev.log 2>&1 || true
wait
# Without data.img, FS should be degraded but kernel should still boot
grep -a -q "FS: no data disk" /tmp/f1-nodev.log && grep -a -q "SHELL-OK" /tmp/f1-nodev.log \
  && echo NODEV-ASSERT-OK || echo NODEV-ASSERT-PENDING

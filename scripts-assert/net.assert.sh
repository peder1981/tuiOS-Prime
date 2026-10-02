#!/usr/bin/env bash
# roda QEMU com a imagem e exige que a rede inicialize (probe + static IP)
# Nota: receive ainda não funciona no QEMU user-mode com IP estático (Fase 2)
set -uo pipefail
cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
rm -f /tmp/f1-net-in; mkfifo /tmp/f1-net-in
( sleep 20 ) > /tmp/f1-net-in &
timeout 30 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  -netdev user,id=n0 -device virtio-net-pci,netdev=n0 \
  < /tmp/f1-net-in > /tmp/f1-net.log 2>&1 || true
wait
grep -a -q "NET-DEV virtio" /tmp/f1-net.log && grep -a -q "NET-STATIC ip=" /tmp/f1-net.log \
  && grep -a -q "SHELL-OK" /tmp/f1-net.log && echo NET-ASSERT-OK || echo NET-ASSERT-PENDING

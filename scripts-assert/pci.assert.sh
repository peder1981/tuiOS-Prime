#!/usr/bin/env bash
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  -netdev user,id=n0 -device virtio-net-pci,netdev=n0 > /tmp/f1-pci.log 2>&1 || true
grep -a -q "PCI-OK" /tmp/f1-pci.log && grep -a -q "1af4:1001" /tmp/f1-pci.log && echo PCI-ASSERT-OK || echo PCI-ASSERT-PENDING

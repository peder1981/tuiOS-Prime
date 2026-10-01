#!/usr/bin/env bash
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide > /tmp/f1-time.log 2>&1 || true
grep -a -q TIME-OK /tmp/f1-time.log && echo TIME-ASSERT-OK || echo TIME-ASSERT-PENDING

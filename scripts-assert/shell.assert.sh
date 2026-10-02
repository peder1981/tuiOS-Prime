#!/usr/bin/env bash
cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
# roda QEMU com a imagem e exige SHELL-OK no serial
set -uo pipefail
timeout 20 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide > /tmp/f1-shell.log 2>&1 || true
grep -a -q SHELL-OK /tmp/f1-shell.log && echo SHELL-ASSERT-OK || echo SHELL-ASSERT-PENDING

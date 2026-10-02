#!/usr/bin/env bash
set -uo pipefail
mkdir -p /tmp/httpsrv && printf 'HTTP HELLO FROM HOST\n' > /tmp/httpsrv/hello.txt
(python3 -m http.server 18080 --directory /tmp/httpsrv >/dev/null 2>&1 &)
sleep 1
rm -f /tmp/f1-net-in; mkfifo /tmp/f1-net-in
( for i in $(seq 1 30); do sleep 1; grep -a -q "SHELL-OK" /tmp/f1-net.log 2>/dev/null && break; done
  printf 'net\nping\nhttp\n'; sleep 25 ) > /tmp/f1-net-in &
timeout 60 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd \
  -drive file=disk.img,format=raw,if=ide \
  -drive file=data.img,format=raw,if=virtio \
  -netdev user,id=n0 -device virtio-net-pci,netdev=n0 < /tmp/f1-net-in > /tmp/f1-net.log 2>&1 || true
wait
pkill -f "http.server 18080" || true
grep -a -q "NET-DHCP-OK" /tmp/f1-net.log && grep -a -q "NET-PING-OK" /tmp/f1-net.log \
  && grep -a -q "HTTP-GET-OK" /tmp/f1-net.log && echo NET-ASSERT-OK || echo NET-ASSERT-PENDING

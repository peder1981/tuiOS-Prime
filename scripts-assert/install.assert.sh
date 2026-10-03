#!/usr/bin/env bash
# install.assert.sh — T1: instalação automatizada em QEMU; T2: boot do disco instalado.
# Saída: INSTAL-OK (instalador) e INSTAL-TEST-OK (script, exit 0).
set -euo pipefail
cd "$(dirname "$0")/.."

ISO="${ISO:-}"
if [ -z "$ISO" ]; then
  for c in result/iso/*.iso; do
    [ -f "$c" ] && ISO="$c" && break
  done
fi
if [ -z "$ISO" ] && { [ -L result ] || [ -e result ]; }; then
  # out-link pode apontar para /nix/store lógico (build com --store local)
  REAL=$(readlink -f result 2>/dev/null || true)
  case "$REAL" in
    /nix/store/*) [ -f "$REAL" ] || REAL="/tmp/nix-official$REAL" ;;
  esac
  ISO=$(ls "$REAL"/iso/*.iso 2>/dev/null | head -1 || true)
fi
if [ -z "$ISO" ]; then
  for c in /tmp/tuios-prime-iso-oficial/*.iso; do
    [ -f "$c" ] && ISO="$c" && break
  done
fi
[ -f "${ISO:-}" ] || { echo "ISO não encontrada — rode: just iso"; exit 1; }
echo "ISO: $ISO"

OVMF_CODE="${OVMF_CODE:-/usr/share/OVMF/OVMF_CODE_4M.fd}"
OVMF_VARS="${OVMF_VARS:-/usr/share/OVMF/OVMF_VARS_4M.fd}"
[ -f "$OVMF_CODE" ] && [ -f "$OVMF_VARS" ] || { echo "OVMF ausente (pacote ovmf)"; exit 1; }

WORK=$(mktemp -d /tmp/tuios-install-test.XXXXXX)
DISK="$WORK/disk.qcow2"
qemu-img create -qf qcow2 "$DISK" 8G >/dev/null
mkfifo "$WORK/in"

QPID=""
cleanup() { [ -n "$QPID" ] && kill "$QPID" 2>/dev/null || true; }
trap cleanup EXIT

boot() { # $1 = iso ou "null"
  cp "$OVMF_VARS" "$WORK/vars.fd"
  local cdrom=()
  [ "$1" != "null" ] && cdrom=(-cdrom "$1")
  : > "$WORK/log"   # criar antes: bash abre redirects em ordem; fifo bloqueia
  # KVM quando disponível (TCG é ~10x mais lento — pode estourar timeout);
  # -cpu host dá RDRAND (entropia). Fallback: TCG puro.
  local accel=()
  if [ -w /dev/kvm ]; then accel=(-enable-kvm -cpu host); fi
  qemu-system-x86_64 "${accel[@]}" -M q35 -m 3072M -smp 4 \
    -display none -serial stdio \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file="$WORK/vars.fd" \
    -drive file="$DISK",format=qcow2,if=virtio \
    -device virtio-rng-pci \
    "${cdrom[@]}" \
    -netdev user,id=n0 -device virtio-net-pci,netdev=n0 \
    > "$WORK/log" 2>&1 < "$WORK/in" &
  QPID=$!
  # Escritor persistente: sem isto, o QEMU fica bloqueado abrindo a fifo
  # para leitura e nunca exec (deadlock com o aguardo do prompt).
  if [ -z "${FIFO_WR:-}" ]; then
    exec 8> "$WORK/in"
    FIFO_WR=1
  fi
}

esperar() { # $1=padrão $2=timeout_s
  local i
  for i in $(seq 1 "$2"); do
    grep -a -q "$1" "$WORK/log" 2>/dev/null && return 0
    kill -0 "$QPID" 2>/dev/null || { echo "QEMU morreu — veja $WORK/log"; return 1; }
    sleep 1
  done
  echo "TIMEOUT esperando: $1 — últimas linhas:"
  tr -d '\000-\010\013-\037' < "$WORK/log" | sed 's/\[[0-9;]*[mKH]//g' | tail -20
  return 1
}

enviar() { printf '%s\n' "$1" >&8; }

# strip cobre também escapes tipo [?2004h/l (bracketed paste) e [0m
limpar_log() { tr -d '\000-\010\013-\037' < "$WORK/log" | sed 's/\[[0-9;?]*[a-zA-Z]//g'; }

# Prompt real do bash (PS1 com "]#" — não "~#")
PROMPT='root@tuios-prime:~]#'

echo "[1/2] Boot da ISO + instalação automatizada..."
boot "$ISO"
esperar "$PROMPT" 600
sleep 3
enviar "tuios-instalar --auto --disco /dev/vda --sem-rede --aceitar-tudo; echo RC=\$?"
esperar "INSTAL-OK" 3600
grep -a -q "INSTAL-OK" "$WORK/log" || { echo "FALHA: INSTAL-OK ausente"; exit 1; }
grep -a -q "RC=0" "$WORK/log" || { echo "FALHA: instalador retornou != 0"; limpar_log | tail -30; exit 1; }
kill "$QPID" 2>/dev/null || true; wait "$QPID" 2>/dev/null || true; QPID=""

echo "[2/2] Boot do disco instalado..."
boot null
esperar "$PROMPT" 900
sleep 3
enviar "systemctl is-active tuios-session; hostname; advplc --version 2>&1 | head -1; tuios-apps listar --json; echo T2-FIM"
esperar "T2-FIM" 60
sleep 1
limpar_log | grep -q "^active$" || { echo "FALHA: tuios-session inativo"; limpar_log | tail -30; exit 1; }
limpar_log | grep -q "^tuios-prime$" || { echo "FALHA: hostname errado"; exit 1; }
# aceita "advplc dev" (sem ldflags) ou "advplc vX.Y.Z" (release com versão injetada)
limpar_log | grep -qE "^advplc (dev|v[0-9]+\.)" || { echo "FALHA: advplc ausente"; exit 1; }
# tuios-apps instalado com o exemplo de fabrica (R18 — veio da live via cp do instalador)
limpar_log | grep -q '"nome": "ola-tuios"' || { echo "FALHA: ola-tuios nao listado no T2"; limpar_log | tail -30; exit 1; }
limpar_log | grep -q '"origem": "sistema"' || { echo "FALHA: ola-tuios nao veio como sistema"; limpar_log | tail -30; exit 1; }

kill "$QPID" 2>/dev/null || true; wait "$QPID" 2>/dev/null || true; QPID=""
echo ""
echo "============================================"
echo "   INSTAL-TEST-OK (T1 instalação + T2 boot)"
echo "============================================"
rm -rf "$WORK"

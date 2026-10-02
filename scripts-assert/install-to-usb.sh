#!/usr/bin/env bash
# install-to-usb.sh — Instala tuiOS-Prime em pendrive USB
# Uso: sudo bash scripts-assert/install-to-usb.sh /dev/sdX

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Uso: sudo $0 <dispositivo>"
    echo "Exemplo: sudo $0 /dev/sdb"
    echo ""
    echo "Dispositivos disponiveis:"
    lsblk -d -o NAME,SIZE,TYPE,MODEL 2>/dev/null | head -20
    exit 1
fi

DEVICE="$1"

if [ ! -b "$DEVICE" ]; then
    echo "Erro: $DEVICE não é um dispositivo bloco válido!"
    exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
    echo "Erro: Este script precisa ser executado como root (sudo)"
    exit 1
fi

echo "========================================"
echo "   tuiOS-Prime — Instalação no USB"
echo "========================================"
echo ""
echo "Dispositivo: $DEVICE"
echo "Imagem: disk.img ($(du -h disk.img | cut -f1))"
echo ""

# Gravar imagem
echo "Gravando imagem..."
dd if=disk.img of="$DEVICE" bs=4M status=progress conv=fsync

# Sincronizar
sync

echo ""
echo "========================================"
echo "   ✅ INSTALAÇÃO CONCLUÍDA!"
echo "========================================"
echo ""
echo "Para bootar:"
echo "1. Insira o pendrive no Chromebook"
echo "2. Desligue o dispositivo"
echo "3. Segure Esc + Refresh + Power"
echo "4. Quando aparecer tela vermelha, pressione Ctrl+D"
echo "5. Selecione o pendrive USB no menu de boot"
echo ""
echo "Ou segure Ctrl+U na inicialização para menu de boot."
echo ""

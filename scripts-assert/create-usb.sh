#!/usr/bin/env bash
# create-usb.sh — Gera pendrive bootavel do tuiOS-Prime
# Uso: sudo bash scripts-assert/create-usb.sh /dev/sdX
# Exemplo: sudo bash scripts-assert/create-usb.sh /dev/sdb

set -euo pipefail

# Verificar argumentos
if [ $# -ne 1 ]; then
    echo "Uso: sudo $0 <dispositivo>"
    echo "Exemplo: sudo $0 /dev/sdb"
    echo ""
    echo "Dispositivos disponiveis:"
    lsblk -d -o NAME,SIZE,TYPE,MODEL 2>/dev/null | head -20
    exit 1
fi

DEVICE="$1"

# Verificar se é um dispositivo valido
if [ ! -b "$DEVICE" ]; then
    echo "Erro: $DEVICE não é um dispositivo bloco valido!"
    echo ""
    echo "Dispositivos disponiveis:"
    lsblk -d -o NAME,SIZE,TYPE 2>/dev/null
    exit 1
fi

# Verificar se tem privilégios
if [ "$(id -u)" -ne 0 ]; then
    echo "Erro: Este script precisa ser executado como root (sudo)"
    exit 1
fi

# Verificar se o dispositivo não está montado
if mount | grep -q "^$DEVICE"; then
    echo "Erro: $DEVICE está montado. Desmonte antes de continuar."
    mount | grep "^$DEVICE"
    exit 1
fi

# Verificar particoes
if lsblk -o TYPE "$DEVICE" 2>/dev/null | grep -q part; then
    echo "Aviso: $DEVICE possui particoes. Todas serao apagadas!"
    echo ""
    read -p "Continuar? (s/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Ss]$ ]]; then
        echo "Cancelado."
        exit 1
    fi
fi

echo "========================================"
echo "   tuiOS-Prime — Criar Pendrive Bootavel"
echo "========================================"
echo ""
echo "Dispositivo: $DEVICE"
echo "Imagem: disk.img"
echo ""

# Verificar se a imagem existe
if [ ! -f "disk.img" ]; then
    echo "Error: disk.img não encontrada. Execute 'just image' primeiro."
    exit 1
fi

# Verificar tamanho
IMG_SIZE=$(stat -c%s disk.img 2>/dev/null || stat -f%z disk.img 2>/dev/null)
DEV_SIZE=$(blockdev --getsize64 "$DEVICE" 2>/dev/null || echo "0")

if [ "$DEV_SIZE" -eq 0 ] || [ "$IMG_SIZE" -gt "$DEV_SIZE" ]; then
    echo "Erro: Dispositivo muito pequeno!"
    echo "  Tamanho da imagem: $((IMG_SIZE / 1024 / 1024)) MB"
    echo "  Tamanho do dispositivo: $((DEV_SIZE / 1024 / 1024)) MB"
    exit 1
fi

echo "Tamanho da imagem: $((IMG_SIZE / 1024 / 1024)) MB"
echo "Tamanho do dispositivo: $((DEV_SIZE / 1024 / 1024)) MB"
echo ""

# Gravar imagem
echo "Gravando imagem no pendrive..."
dd if=disk.img of="$DEVICE" bs=4M status=progress conv=fsync

# Sincronizar
sync

echo ""
echo "========================================"
echo "   ✅ PENDRIVE CRIADO COM SUCESSO!"
echo "========================================"
echo ""
echo "Próimos passos:"
echo "1. Remova com segurança: sudo eject $DEVICE"
echo "2. No Chromebook, segure Ctrl+U na inicialização"
echo "3. Selecione o pendrive USB no menu de boot"
echo ""
echo "Instalação no Chromebook:"
echo "  - Modo desenvolvedor: Esc + Refresh + Power"
echo "  - Habilitar boot USB: sudo crossystem dev_boot_usb=1"
echo "  - Boot: Ctrl+U na inicialização"
echo ""

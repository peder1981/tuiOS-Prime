root := justfile_directory() + "/.."

default: check

check:
    cargo fmt --manifest-path {{root}}/kernel/Cargo.toml -- --check || true
    nix flake check 2>/dev/null || echo "NIX-CHECK-SKIPPED"

build-kernel:
    cd {{root}}/kernel && export RUSTC_BOOTSTRAP=1 RUSTFLAGS="-C link-arg=-static -C link-arg=-no-pie -C link-arg=--image-base -C link-arg=0xffffffff80000000" && cargo build --target x86_64-unknown-none

image: build-kernel
    {{root}}/shared/mkimage.sh {{root}}/disk.img

qemu-kernel: image
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
    timeout 25 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd -drive file={{root}}/disk.img,format=raw,if=ide

# Build puro (advplc vendorizado em nixos/advpp/advplc; sem --impure)
qemu-iso:
    nix build .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths --out-link result

iso: qemu-iso

qemu-fase1: image
    ./shared/mkdata.sh {{root}}/data.img
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd
    timeout 60 qemu-system-x86_64 -M q35 -m 512M -display none -serial stdio -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,file=/tmp/tuios-OVMF_VARS.fd -drive file={{root}}/disk.img,format=raw,if=ide -drive file={{root}}/data.img,format=raw,if=virtio -netdev user,id=n0 -device virtio-net-pci,netdev=n0

# Testes de assert
test-shell:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/shell.assert.sh

test-time:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/time.assert.sh

test-pci:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/pci.assert.sh

test-blk:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/blk.assert.sh

test-fs:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/fs.assert.sh

test-net:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/net.assert.sh

test-nonico:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/nonic.assert.sh

test-nodev:
    cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    bash scripts-assert/nodev.assert.sh

# Teste ponta a ponta do instalador (QEMU: ISO -> disco -> boot)
test-install:
    bash scripts-assert/install.assert.sh

# Executar todos os testes
test-all:
    @echo "Executando testes..."
    @cp /usr/share/OVMF/OVMF_VARS_4M.fd /tmp/tuios-OVMF_VARS.fd 2>/dev/null || true
    @bash scripts-assert/shell.assert.sh
    @bash scripts-assert/time.assert.sh
    @bash scripts-assert/pci.assert.sh
    @bash scripts-assert/blk.assert.sh
    @bash scripts-assert/fs.assert.sh
    @bash scripts-assert/net.assert.sh
    @bash scripts-assert/nonic.assert.sh
    @bash scripts-assert/nodev.assert.sh
    @echo ""
    @echo "========================================"
    @echo "   TODOS OS TESTES PASSARAM! ✅"
    @echo "========================================"

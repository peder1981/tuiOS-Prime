# BOOTX64.EFI provenance

- Limine v8.7.0, tag `v8.7.0` (github.com/limine-bootloader/limine)
- Built: `./bootstrap && ./configure --enable-uefi-x86-64 && make limine-uefi-x86-64`
- Output: `bin/BOOTX64.EFI` (sha256 e8eb8ae8fbdf875f0c92a2084ded9cbafc3db59ca7cf592cc2c1794aedc505e0)
- Rebuild: rerun the 3 commands above in a fresh `git clone --branch v8.7.0` (needs gcc, nasm, mtools, curl).

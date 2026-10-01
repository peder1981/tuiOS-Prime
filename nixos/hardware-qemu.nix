{ modulesPath, ... }: {
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];
  boot.initrd.availableKernelModules = [ "virtio_blk" "virtio_net" "virtio_pci" "ahci" ];
  networking.useDHCP = true;
}

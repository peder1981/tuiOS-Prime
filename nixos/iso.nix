{ modulesPath, pkgs, ... }: {
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
    ./hardware-qemu.nix
  ];
  networking.hostName = "tuios-prime";
  services.getty.autologinUser = "root";
  systemd.services.tuios-session = {
    description = "tuiOS fullscreen session";
    after = [ "getty@tty1.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${pkgs.bash}/bin/bash -lc 'exec tuios attach || exec tuios'";
      StandardInput = "tty";
      TTYPath = "/dev/tty1";
      TTYReset = true;
      TTYVHangup = true;
    };
  };
  environment.systemPackages = with pkgs; [ htop pciutils usbutils ];
}

{ modulesPath, pkgs, tuios, advplc, ... }:

{
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
    ./hardware-qemu.nix
  ];

  networking.hostName = "tuios-prime";

  # Login automático como root no console
  services.getty.autologinUser = "root";

  # Sessão tuiOS — inicia automaticamente no boot (tty1, tela cheia).
  # Tenta anexar a uma sessão existente; senão, inicia uma nova.
  systemd.services.tuios-session = {
    description = "Sessão tuiOS em tela cheia";
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

  # Pacotes do sistema
  environment.systemPackages = [
    # Interface de terminal tuiOS (do flake input)
    tuios.packages.${pkgs.system}.default

    # Compilador AdvPL/TLPP (do pacote local ./advpp)
    advplc

    # Utilitários
    pkgs.htop
    pkgs.pciutils
    pkgs.usbutils
    pkgs.vim
    pkgs.git
    pkgs.go
  ];

  # Banco de dados compartilhado do AdvPP (ver `advplc --help`)
  environment.variables = {
    ADVPP_DB = "/var/lib/advpp/advpp.db";
  };

  systemd.tmpfiles.rules = [
    "d /var/lib/advpp 0755 root root -"
  ];
}

{ modulesPath, pkgs, lib, tuios, advplc, ... }:

{
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
    ./hardware-qemu.nix
    ./tuios-session.nix
  ];

  networking.hostName = "tuios-prime";

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

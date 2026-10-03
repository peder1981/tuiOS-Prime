# Configuração do sistema tuiOS-Prime INSTALADO em disco.
# Gerada pelo tuios-instalar; pós-instalação: nixos-rebuild switch.
{ config, pkgs, lib, ... }:
let
  env = import ./tuios-env.nix;
  installer = import ./tuios-installer-pkg.nix;
in {
  imports = [
    ./hardware-configuration.nix
    ./tuios-session.nix
  ];

  networking.hostName = "tuios-prime";
  networking.networkmanager.enable = true;

  time.timeZone = "America/Sao_Paulo";
  i18n.defaultLocale = "pt_BR.UTF-8";
  console.keyMap = "br-abnt2";

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = false;

  environment.systemPackages = [
    env.tuios
    env.advplc
    installer.pkg
    pkgs.htop
    pkgs.pciutils
    pkgs.usbutils
    pkgs.vim
    pkgs.git
    pkgs.go
    pkgs.dialog
    pkgs.networkmanager
    pkgs.dialog
  ];

  environment.variables = { ADVPP_DB = "/var/lib/advpp/advpp.db"; };
  systemd.tmpfiles.rules = [ "d /var/lib/advpp 0755 root root -" ];

  system.stateVersion = "24.05";
}

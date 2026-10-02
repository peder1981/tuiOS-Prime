{ modulesPath, pkgs, lib, tuios, advplc, nixpkgsPath, ... }:

{
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
    ./hardware-qemu.nix
    ./tuios-session.nix
  ];

  networking.hostName = "tuios-prime";

  # NetworkManager: já habilitado pelo perfil installation-cd-minimal
  # (o instalador usa nmcli; mantido explícito como mkDefault documental)
  networking.networkmanager.enable = lib.mkDefault true;
  # O perfil minimal liga wpa_supplicant (wireless); NM substitui
  networking.wireless.enable = false;

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

  # ---------- Material do instalador embutido (/etc/tuios-installer/) ----------
  environment.etc."tuios-installer/configuration.nix".source = ./instalado/configuration.nix;
  environment.etc."tuios-installer/tuios-session.nix".source = ./tuios-session.nix;
  # Store paths de tuios/advplc para o eval do destino (storePath mantém contexto)
  environment.etc."tuios-installer/tuios-env.nix".text = ''
    {
      tuios  = builtins.storePath ${tuios.packages.${pkgs.system}.default};
      advplc = builtins.storePath ${advplc};
    }
  '';
  # Store path do próprio instalador para o template do destino
  environment.etc."tuios-installer/tuios-installer-pkg.nix".text = ''
    { pkg = builtins.storePath ${pkgs.callPackage ../installer/package.nix { }}; }
  '';
  # nixpkgs source (NIX_PATH do nixos-install — avaliação offline)
  environment.etc."tuios-installer/nixpkgs-path".text = nixpkgsPath;
}

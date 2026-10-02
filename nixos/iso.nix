{ modulesPath, pkgs, ... }:

{
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
    ./hardware-qemu.nix
    ./advpp
  ];
  
  networking.hostName = "tuios-prime";
  
  # Autologin
  services.getty.autologinUser = "root";
  
  # Session tuiOS
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
  
  # Packages do sistema
  environment.systemPackages = with pkgs; [
    # tuiOS
    tuios.packages.${system}.default
    
    # AdvPP (compilador)
    advplc
    
    # Utilitários
    htop
    pciutils
    usbutils
    vim
    git
    go
  ];
  
  # Variaveis de ambiente para AdvPP
  environment.variables = {
    ADVPP_SRC = "/opt/advpp";
    ADVPP_DB = "/var/lib/advpp/advpp.db";
  };
  
  # Criar directories para AdvPP
  systemd.tmpfiles.rules = [
    "d /opt/advpp 0755 root root -"
    "d /var/lib/advpp 0755 root root -"
    "d /home/%u/.advpp 0755 %u %u -"
  ];
  
  # Copiar fonte do AdvPP para o sistema
  installFiles = [
    {
      source = /home/peder/Projetos/AdvPP;
      target = "/opt/advpp";
    }
  ];
}

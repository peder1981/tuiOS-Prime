{ modulesPath, pkgs, lib, tuios, advplc, tuiosApps, nixpkgsPath, nixpkgsSrc, ... }:

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
    # Gerenciador de apps AdvPL (Nível C, fase 1)
    tuiosApps

    # Assistente de instalação
    (pkgs.callPackage ../installer/package.nix { })
    pkgs.dialog
    pkgs.networkmanager

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

  # ---------- Assistente automático no boot da live ----------
  # O usuário não precisa adivinhar o comando: o assistente abre SOZINHO
  # no tty1 antes da sessão tuiOS (o WM sozinho não expõe o instalador).
  # Ao terminar (concluído, falha ou cancelado) a sessão tuiOS assume.
  # O '-' no ExecStartPre garante que cancelamento/falha não derrubam
  # a sessão. Exclusivo da ISO live — o sistema instalado não importa
  # este módulo.
  systemd.services.tuios-session.serviceConfig = {
    # bash -lc: login shell carrega o PATH do systemPackages (tuios-instalar,
    # dialog, clear) — o PATH do serviço systemd sozinho é mínimo.
    ExecStartPre = [ ("-" + "${pkgs.bash}/bin/bash -lc " + pkgs.writeShellScript "tuios-live-autostart" ''
      # DIAGNÓSTICO: só os marcadores vão ao arquivo. NUNCA redirecionar
      # stdout/stderr do script inteiro — dialog renderiza a tela em stderr
      # e o redirect global escondia o wizard dentro do log (tela congelada).
      LOG=/var/log/tuios-live-autostart.log
      echo "== autostart iniciado $(date) ==" >> "$LOG"
      clear
      echo "=========================================================="
      echo "   tuiOS-Prime — Assistente de Instalacao (live)"
      echo ""
      echo "   O assistente vai instalar o sistema neste computador."
      echo "   Para apenas testar, cancele (ESC/Cancelar/NAO)."
      echo "   Depois voce pode reexecuta-lo com:  tuios-instalar"
      echo "=========================================================="
      echo
      ${pkgs.coreutils}/bin/sleep 2
      rc=0
      tuios-instalar || rc=$?
      clear
      echo "== autostart concluido $(date) rc=$rc ==" >> "$LOG"
    '' ) ];
    # dialog no tty1 precisa de TERM e de controlling terminal
    # (tty-force = TCSCTTY — sem ele /dev/tty falha e o wizard morre)
    Environment = [ "TERM=linux" ];
    StandardInput = lib.mkForce "tty-force";
  };

  # ---------- Material do instalador embutido (/etc/tuios-installer/) ----------
  environment.etc."tuios-installer/configuration.nix".source = ./instalado/configuration.nix;
  environment.etc."tuios-installer/lib/ui.sh".source       = ../installer/lib/ui.sh;
  environment.etc."tuios-installer/lib/disco.sh".source    = ../installer/lib/disco.sh;
  environment.etc."tuios-installer/lib/rede.sh".source     = ../installer/lib/rede.sh;
  environment.etc."tuios-installer/lib/locale.sh".source   = ../installer/lib/locale.sh;
  environment.etc."tuios-installer/lib/instalar.sh".source = ../installer/lib/instalar.sh;
  environment.etc."tuios-installer/tuios-session.nix".source = ./tuios-session.nix;
  # Store paths de tuios/advplc para o eval do destino (storePath mantém contexto)
  environment.etc."tuios-installer/tuios-env.nix".text = ''
    {
      tuios  = builtins.storePath ${tuios.packages.${pkgs.system}.default};
      advplc = builtins.storePath ${advplc};
      tuiosApps = builtins.storePath ${tuiosApps};
    }
  '';
  # Store path do próprio instalador para o template do destino
  environment.etc."tuios-installer/tuios-installer-pkg.nix".text = ''
    { pkg = builtins.storePath ${pkgs.callPackage ../installer/package.nix { }}; }
  '';
  # nixpkgs source (NIX_PATH do nixos-install — avaliação offline).
  # Interpolação do input (nixpkgsSrc) gera contexto → a source ENTRA no
  # closure da ISO; outPath cru (string sem contexto) NÃO entraria.
  environment.etc."tuios-installer/nixpkgs-path".text = "${nixpkgsSrc}";
}

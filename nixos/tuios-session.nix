{ pkgs, lib, ... }:
# Sessão tuiOS + autologin root — compartilhado entre a ISO (live) e o
# sistema instalado. tty1 exclusivo da sessão (getty disputava o terminal
# e causava SIGHUP no tuios — ver histórico).
{
  boot.kernelParams = [ "console=ttyS0,115200n8" ];

  services.getty.autologinUser = lib.mkForce "root";
  systemd.services."getty@tty1".enable = false;
  systemd.services."autovt@tty1".enable = false;

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
}

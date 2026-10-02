{ lib, stdenv, makeWrapper
, dialog, networkmanager, parted, dosfstools, e2fsprogs, gptfdisk
, util-linux, coreutils, findutils, gawk, gnugrep, systemd }:

stdenv.mkDerivation {
  pname = "tuios-instalar";
  version = "1.0.0";
  src = ./.;

  dontConfigure = true;
  dontBuild = true;

  nativeBuildInputs = [ makeWrapper ];

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin $out/lib
    cp tuios-instalar $out/bin/
    cp -r lib/. $out/lib/
    chmod +x $out/bin/tuios-instalar
    runHook postInstall
  '';

  postFixup = ''
    wrapProgram $out/bin/tuios-instalar --prefix PATH : ${
      lib.makeBinPath [ dialog networkmanager parted dosfstools e2fsprogs
                        gptfdisk util-linux coreutils findutils gawk gnugrep systemd ]
    }
  '';
}

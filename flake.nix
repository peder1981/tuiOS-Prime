{
  description = "tuiOS-Prime — dual-track Rust kernel + NixOS ISO + tuiOS + AdvPP";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    # Fork do tuiOS (terminal UI). Usa o repositório remoto para que o
    # build seja reproduzível em qualquer máquina; para desenvolvimento
    # local, sobrescreva com:
    #   --override-input tuios path:/home/peder/Projetos/tuiOS
    tuios.url = "github:peder1981/tuiOS";
  };

  outputs = { self, nixpkgs, tuios, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      # Compilador AdvPP (binário pré-compilado local, ver nixos/advpp).
      advplc = pkgs.callPackage ./nixos/advpp { };
      # Gerenciador de apps AdvPL (Nível C, fase 1 — ver apps/tuios-apps)
      tuiosApps = pkgs.callPackage ./nixos/apps/tuios-apps.nix { };
      # App-exemplo de fabrica do Nivel C (spec Fase 2, R16)
      tuiosExemplos = pkgs.callPackage ./nixos/apps/exemplos.nix { };
    in {
      # Shell de desenvolvimento
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          just
          qemu
          ovmf
          rustup
          go
        ];

        SHELL = "/bin/bash";
      };

      # Pacotes disponíveis
      packages.${system} = {
        # Interface de terminal tuiOS
        tuios = tuios.packages.${system}.default;
        # Compilador AdvPL/TLPP
        inherit advplc;
        # Gerenciador de apps AdvPL
        inherit tuiosApps;
        # App-exemplo de fabrica
        inherit tuiosExemplos;
        default = self.packages.${system}.tuios;
      };

      # Configuração da ISO NixOS
      nixosConfigurations.iso = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit tuios advplc tuiosApps tuiosExemplos; nixpkgsPath = nixpkgs.outPath; nixpkgsSrc = nixpkgs; };
        modules = [
          ./nixos/iso.nix
        ];
      };
    };
}

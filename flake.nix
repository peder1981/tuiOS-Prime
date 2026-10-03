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
        default = self.packages.${system}.tuios;
      };

      # Configuração da ISO NixOS
      nixosConfigurations.iso = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit tuios advplc; nixpkgsPath = nixpkgs.outPath; nixpkgsSrc = nixpkgs; };
        modules = [
          ./nixos/iso.nix
        ];
      };
    };
}

{
  description = "tuiOS-Prime — dual-track Rust kernel + NixOS ISO + AdvPP + tuiOS";
  
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    # Input do tuiOS (fork local)
    tuios.url = "path:/home/peder/Projetos/tuiOS";
  };
  
  outputs = { self, nixpkgs, tuios, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      # Dev shell
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
      
      # Packages disponiveis
      packages.${system} = {
        # tuiOS terminal UI
        tuios = tuios.packages.${system}.default;
        # Kernel Rust (sera construido via just)
        default = self.packages.${system}.tuios;
      };
      
      # NixOS Configuration
      nixosConfigurations.iso = pkgs.lib.nixosSystem {
        inherit system;
        modules = [
          ./nixos/iso.nix
          ./nixos/advpp
        ];
      };
    };
}

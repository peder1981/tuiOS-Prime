{
  description = "tuiOS-Prime — dual-track Rust kernel + NixOS ISO";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    tuios.url = "github:peder1981/tuiOS";
  };
  outputs = { self, nixpkgs, ... }:
    let system = "x86_64-linux";
    in {
      devShells.${system}.default =
        nixpkgs.legacyPackages.${system}.mkShell {
          packages = with nixpkgs.legacyPackages.${system}; [
            just qemu OVMF rustup
          ];
        };
      nixosConfigurations.iso = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [ ./nixos/iso.nix ];
      };
    };
}

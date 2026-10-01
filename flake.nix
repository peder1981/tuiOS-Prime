{
  description = "tuiOS-Prime — dual-track Rust kernel + NixOS ISO";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    tuios.url = "github:peder1981/tuiOS";
  };
  outputs = { self, nixpkgs, ... }: {
    devShells.x86_64-linux.default =
      nixpkgs.legacyPackages.x86_64-linux.mkShell {
        packages = with nixpkgs.legacyPackages.x86_64-linux; [
          just qemu OVMF rustup
        ];
      };
  };
}

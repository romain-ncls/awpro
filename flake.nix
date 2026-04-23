{
  description = "Rust Rover environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      supportedSystems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
        config.allowUnfree = true;
      };
    in
    {
      packages = forAllSystems (system: {
        awpro = (pkgsFor system).pkgsStatic.callPackage ./default.nix { };
        default = self.packages.${system}.awpro;
      });
      devShells = forAllSystems (system: {
        default = (pkgsFor system).callPackage ./shell.nix { };
      });
    };
}

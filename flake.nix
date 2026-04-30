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
      nixosModules.default = { pkgs, lib, config, ... }:
        let
          udev-rules = pkgs.writeTextFile {
            name = "awpro-udev-rules";
            destination = "/lib/udev/rules.d/70-awpro.rules";
            text = ''
              ACTION!="add|change", GOTO="awpro_end"
              # Dell AW Pro headset (413c:a529)
              SUBSYSTEM=="hidraw", ATTRS{idVendor}=="413c", ATTRS{idProduct}=="a529", TAG+="uaccess"
              LABEL="awpro_end"
            '';
          };
        in
        {
          options.programs.awpro.enable = lib.mkEnableOption "awpro headset tool";
          config = lib.mkIf config.programs.awpro.enable {
            services.udev.packages = [ udev-rules ];
            environment.systemPackages = [ self.packages.${pkgs.system}.awpro ];
          };
        };
      devShells = forAllSystems (system: {
        default = (pkgsFor system).callPackage ./shell.nix { };
      });
    };
}

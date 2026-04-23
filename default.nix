{
  pkgs ? import <nixpkgs> { },
}:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "awpro";
  version = "0.0.1";

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    libudev-zero
  ];

  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.cleanSource ./.;
}

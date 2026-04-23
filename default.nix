{ lib, rustPlatform, libudev-zero, pkg-config }:
rustPlatform.buildRustPackage {
  pname = "awpro";
  version = "0.0.1";

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ libudev-zero ];

  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;
}

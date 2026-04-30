{
  lib,
  rustPlatform,
  libudev-zero,
  pkg-config,
}:
let
  manifest = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ libudev-zero ];

  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;

  postFixup = ''
    rm $out/nix-support/propagated-build-inputs
  '';
}

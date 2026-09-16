{
  lib,
  rustPlatform,
}:
let
  manifest = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;

  # No nativeBuildInputs or buildInputs: hidapi's `linux-native-basic-udev`
  # backend talks to hidraw through ioctls from pure Rust, so the build needs
  # neither a C compiler nor libudev.

  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;
}

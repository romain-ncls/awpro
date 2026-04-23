{
  pkgs ? import <nixpkgs> { },
}:
let
  rustToolchain = pkgs.rust-bin.stable."1.88.0".default.override {
    extensions = [
      "rust-src"
      "clippy"
      "rustfmt"
    ];
  };
in
pkgs.mkShell {
  # Get dependencies from the main package
  inputsFrom = [ (pkgs.callPackage ./default.nix { }) ];

  nativeBuildInputs = [
    rustToolchain
  ];

  buildInputs = with pkgs; [
    openssl
    pkg-config
    libudev-zero

    rust-analyzer
    jetbrains.rust-rover
    fish
  ];

  shellHook = ''
    # mkdir -p ~/.rust-rover/toolchain

    # ln -sfn ${rustToolchain}/lib ~/.rust-rover/toolchain
    # ln -sfn ${rustToolchain}/bin ~/.rust-rover/toolchain

    export RUST_SRC_PATH="$HOME/.rust-rover/toolchain/lib/rustlib/src/rust/library"
    export SHELL="${pkgs.fish}/bin/fish"
    exec fish
  '';
}

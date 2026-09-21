{
  lib,
  rustPlatform,
  pkg-config,
  alsa-lib,
}:
rustPlatform.buildRustPackage {
  pname = "ttygate";
  version = "0.1.0";

  nativeBuildInputs = [pkg-config];
  buildInputs = [alsa-lib];

  # Rooted at pkgs/ so the ttycanvas path dependency resolves inside the store
  # copy; sourceRoot puts cargo back in this package.
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ../ttycanvas/Cargo.toml
      ../ttycanvas/src
    ];
  };

  sourceRoot = "source/ttygate";

  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    description = "0xc000022070's greeter - a ctOS-flavored ratatui frontend for greetd";
    mainProgram = "ttygate";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}

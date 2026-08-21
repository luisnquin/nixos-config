{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "xgreeter";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    description = "0xc000022070's greeter - a ctOS-flavored ratatui frontend for greetd";
    mainProgram = "xgreeter";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}

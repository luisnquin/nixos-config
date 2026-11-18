{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "winnow";
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
    description = "inline fuzzy picker over stdin lines";
    mainProgram = "winnow";
    license = lib.licenses.mit;
  };
}

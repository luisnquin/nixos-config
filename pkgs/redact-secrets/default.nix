{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "redact-secrets";
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
    description = "Runs a shell command with secrets masked out of its stdout and stderr";
    mainProgram = "redact-secrets";
    license = lib.licenses.mit;
    platforms = lib.platforms.unix;
  };
}

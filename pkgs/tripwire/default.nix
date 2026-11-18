{
  lib,
  rustPlatform,
  callPackage,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "tripwire";
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

  passthru.tests.vm = callPackage ./test.nix {tripwire = finalAttrs.finalPackage;};

  meta = {
    description = "denies every open of a decoy file and freezes the process that tried";
    mainProgram = "tripwire";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
})

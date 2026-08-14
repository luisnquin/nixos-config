{
  lib,
  rustPlatform,
  makeWrapper,
  nix,
}:
rustPlatform.buildRustPackage {
  pname = "heft";
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

  nativeBuildInputs = [makeWrapper];

  # the nix collector shells out for the GC root list
  postInstall = ''
    wrapProgram $out/bin/heft --prefix PATH : ${lib.makeBinPath [nix]}
  '';

  meta = {
    description = "disk intelligence: where the space went, and what changed";
    mainProgram = "heft";
    license = lib.licenses.mit;
  };
}

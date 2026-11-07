{
  lib,
  rustPlatform,
  makeWrapper,
  git,
  lazygit,
}:
rustPlatform.buildRustPackage {
  pname = "drift";
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

  postInstall = ''
    wrapProgram $out/bin/drift --suffix PATH : ${lib.makeBinPath [git lazygit]}
  '';

  meta = {
    description = "how far each repo of a superproject drifted from its upstream";
    mainProgram = "drift";
    license = lib.licenses.mit;
  };
}

{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "herdr-autoname";
  version = "0.5.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./herdr-plugin.toml
      ./shell
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  postInstall = ''
    cp herdr-plugin.toml $out/
    install -Dm644 shell/hook.zsh $out/shell/hook.zsh
  '';

  meta.mainProgram = "herdr-autoname";
}

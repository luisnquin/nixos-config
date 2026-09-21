{
  lib,
  rustPlatform,
  systemd,
  pkg-config,
  alsa-lib,
}:
rustPlatform.buildRustPackage {
  pname = "pinentry-gate";
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

  sourceRoot = "source/pinentry-gate";

  cargoLock.lockFile = ./Cargo.lock;

  # gpg-agent execs the pinentry with the user manager's environment, which
  # carries no PATH worth trusting.
  postPatch = ''
    substituteInPlace src/seat.rs \
      --replace-fail '@loginctl@' '${lib.getExe' systemd "loginctl"}'
  '';

  meta = {
    description = "a pinentry that asks on every surface at once";
    mainProgram = "pinentry-gate";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}

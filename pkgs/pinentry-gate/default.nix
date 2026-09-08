{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "pinentry-gate";
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

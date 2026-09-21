{
  lib,
  rustPlatform,
  systemd,
}:
rustPlatform.buildRustPackage {
  pname = "outage";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
      ./tests
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  postPatch = ''
    substituteInPlace src/effects.rs \
      --replace-fail '@loginctl@' '${lib.getExe' systemd "loginctl"}' \
      --replace-fail '@systemctl@' '${lib.getExe' systemd "systemctl"}'
  '';

  meta = {
    description = "one-shot power-outage protocol: terminate the session, sleep between internet checks";
    mainProgram = "outage";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}

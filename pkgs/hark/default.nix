{
  dbus,
  lib,
  mako,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "hark";
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

  # Both helpers are baked in rather than looked up on PATH: the binary runs
  # from Waybar and from eww button handlers, neither of which inherits a
  # session PATH worth trusting.
  postPatch = ''
    substituteInPlace src/mako.rs \
      --replace-fail '@makoctl@' '${lib.getExe' mako "makoctl"}'
    substituteInPlace src/watch.rs \
      --replace-fail '@dbus_monitor@' '${lib.getExe' dbus "dbus-monitor"}'
  '';

  meta = {
    description = "notification centre over mako's history";
    mainProgram = "hark";
    license = lib.licenses.mit;
  };
}

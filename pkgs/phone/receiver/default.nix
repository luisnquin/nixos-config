# The far side of `phone`: what runs on the mac that owns a simulator, driven
# over ssh by the CLI one directory up.
{
  lib,
  apple-sdk_26,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "phone-receiver";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./build.rs
      ./native
      ./native_stubs.c
      ./src
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  # build.rs compiles the ObjC bridge against CoreSimulator and SimulatorKit,
  # which the default SDK is too old to describe.
  buildInputs = [apple-sdk_26];

  meta = {
    description = "Read and press an iOS Simulator on the host that owns it";
    # The name the controlling host looks for. `phone` on that host is the
    # CLI itself now, which is what drives this one.
    mainProgram = "phone-receiver";
    license = lib.licenses.asl20;
    platforms = ["aarch64-darwin"];
    sourceProvenance = [lib.sourceTypes.fromSource];
  };
}

{
  lib,
  stdenv,
  zig_0_16,
}:
stdenv.mkDerivation {
  pname = "cliphizt";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./build.zig
      ./build.zig.zon
      ./src
      ./vendor
    ];
  };

  nativeBuildInputs = [zig_0_16.hook];

  zigBuildFlags = ["-Doptimize=ReleaseSafe"];

  meta = {
    description = "Wayland clipboard history manager with TTL and ephemeral mode";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "cliphizt";
  };
}

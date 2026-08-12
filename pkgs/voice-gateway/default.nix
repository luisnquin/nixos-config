{
  lib,
  buildGoModule,
}:
buildGoModule {
  pname = "voice-gateway";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./go.mod
      (lib.fileset.fileFilter (f: f.hasExt "go") ./.)
    ];
  };

  vendorHash = null;

  env.CGO_ENABLED = 0;
  ldflags = ["-s" "-w"];

  meta.mainProgram = "voice-gateway";
}

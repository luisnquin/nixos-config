{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:
buildGoModule rec {
  pname = "linear-tui";
  version = "0.10.0";

  src = fetchFromGitHub {
    owner = "roeyazroel";
    repo = "linear-tui";
    tag = "v${version}";
    hash = "sha256-kfDC2AVGJVilxcMWOnz+XvWBqOVFkt+ho8WhQWFQSY4=";
  };

  vendorHash = "sha256-+yC22fb6GtfAXLCIwwSXNRV7FIpelSx25KVa8NiD3Ew=";

  # The repo tracks a prebuilt ./main at the root; only the real entrypoint
  # is worth compiling.
  subPackages = ["cmd/linear-tui"];

  env.CGO_ENABLED = 0;

  ldflags = [
    "-s"
    "-w"
    "-X=main.Version=${version}"
  ];

  postInstall = ''
    mv $out/bin/linear-tui $out/bin/linear
  '';

  meta = {
    description = "Terminal user interface for Linear";
    homepage = "https://github.com/roeyazroel/linear-tui";
    license = lib.licenses.mit;
    mainProgram = "linear";
  };
}

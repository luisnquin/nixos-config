{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:
buildGoModule rec {
  pname = "spiceedit";
  version = "0.0.43";

  src = fetchFromGitHub {
    owner = "cloudmanic";
    repo = "spice-edit";
    tag = "v${version}";
    hash = "sha256-SJ/q7mg6toKbYJjSl1uFH79LR6auxUxguGuXW3kAiDs=";
  };

  vendorHash = "sha256-rjmk+9Yz3riXfvCERs6noGuVOFyEt8SoHbxjAt7D2IY=";

  env.CGO_ENABLED = 0;
  ldflags = ["-s" "-w"];

  postInstall = ''
    mv $out/bin/spice-edit $out/bin/spiceedit
  '';

  meta = {
    description = "Opinionated mouse-first terminal code editor";
    homepage = "https://github.com/cloudmanic/spice-edit";
    license = lib.licenses.mit;
    mainProgram = "spiceedit";
  };
}

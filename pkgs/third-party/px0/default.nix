{
  lib,
  buildGoModule,
  fetchFromGitHub,
  makeWrapper,
  git,
}:
buildGoModule rec {
  pname = "px0";
  version = "0.1.4";

  src = fetchFromGitHub {
    owner = "px0-ai";
    repo = "px0";
    tag = "v${version}";
    hash = "sha256-1r000izgL1EFsBpIkyb8u6+RVhV58qHf+AshSYtINmY=";
  };

  vendorHash = "sha256-71+6I0u3en/Aw3PVMXx6dF+NQtCiE1T+kd7MENCKnlk=";

  env.CGO_ENABLED = 0;
  ldflags = ["-s" "-w"];

  patches = [
    ./disable-auto-update.patch
    ./remove-telemetry.patch
  ];

  nativeBuildInputs = [makeWrapper];
  nativeCheckInputs = [git];

  # Git awareness shells out to git; the suffix leaves a git already on PATH in
  # charge of the user's own config.
  postInstall = ''
    wrapProgram $out/bin/px0 --suffix PATH : ${lib.makeBinPath [git]}
  '';

  meta = {
    description = "Read-only IDE for code navigation and review in the browser";
    homepage = "https://px0.ai";
    license = lib.licenses.mit;
    mainProgram = "px0";
  };
}

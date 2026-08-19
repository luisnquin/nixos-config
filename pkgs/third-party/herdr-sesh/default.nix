{
  buildGoModule,
  fetchFromGitHub,
}:
buildGoModule rec {
  pname = "herdr-sesh";
  version = "0.5.0";

  src = fetchFromGitHub {
    owner = "fullerzz";
    repo = "herdr-plugin-sesh";
    tag = "v${version}";
    hash = "sha256-IGLMExUtNI8ybwY0tOVzhxZSFl5SJgu98DW+kvcBTyY=";
  };

  vendorHash = "sha256-TnfuQetN3KaRsB5r1bTCcQwOw6kqYVjzKb2aWkz6C0A=";

  subPackages = ["cmd/herdr-sesh"];

  ldflags = ["-X=github.com/fullerzz/herdr-plugin-sesh/internal/app.Version=${version}"];

  postInstall = ''
    cp herdr-plugin.toml $out/
  '';

  meta.mainProgram = "herdr-sesh";
}

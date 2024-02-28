{
  fetchFromGitHub,
  buildGoModule,
}:
buildGoModule rec {
  pname = "ecsview";
  version = "v0.1.3";
  src = fetchFromGitHub {
    owner = "swartzrock";
    repo = pname;
    rev = "v0.1.3";
    hash = "sha256-54PR4spA9d/M3G2BqcVFIvMyxPTF3Sc7q4dgCzD1O+I=";
  };

  ldflags = ["-X main.version=${version}"];
  buildTarget = ".";

  vendorHash = "sha256-K5UHOpAwnAHubzFcaQxrPdad0QzvZGcP39/INYgK6EQ=";
  doCheck = false;
}

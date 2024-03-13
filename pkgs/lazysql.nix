{
  fetchFromGitHub,
  buildGoModule,
  pkg-config,
  xorg,
}:
buildGoModule rec {
  pname = "lazysql";
  version = "0.1.8";

  src = fetchFromGitHub {
    owner = "jorgerojas26";
    repo = pname;
    rev = "v${version}}";
    hash = "sha256-yPf9/SM4uET/I8FsDU1le9JgxELu0DR9k7mv8PnBwvQ=";
  };

  buildInputs = [pkg-config xorg.libX11];

  ldflags = ["-X main.version=${version}"];

  vendorHash = "sha256-tgD6qoCVC1ox15VPJWVvhe4yg3R81ktMuW2dsaU69rY=";
  # doCheck = false;
}

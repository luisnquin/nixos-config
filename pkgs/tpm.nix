{
  fetchFromGitHub,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation rec {
  pname = "tpm";
  version = "3.1.0";

  src = fetchFromGitHub {
    owner = "tmux-plugins";
    repo = "tpm";
    rev = "v${version}";
    sha256 = "18i499hhxly1r2bnqp9wssh0p1v391cxf10aydxaa7mdmrd3vqh9";
  };

  preConfigure = ''
    substituteInPlace ./tpm \
      --replace 'CURRENT_DIR=' 'CURRENT_DIR="${placeholder "out"}/share" #'
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/{share,bin}
    cp -r ./{bin,bindings,scripts} $out/share
    cp -r ./tpm $out/bin
    chmod +x $out/bin/*

    runHook postInstall
  '';
}

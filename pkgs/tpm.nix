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
    substituteInPlace {./tpm,./bindings/*,./scripts/*.sh} \
      --replace 'CURRENT_DIR=' 'CURRENT_DIR="${placeholder "out"}/share" # '

    substituteInPlace {./bin/*,./bindings/*} \
     --replace 'SCRIPTS_DIR=' 'SCRIPTS_DIR="${placeholder "out"}/share/scripts" # '

    substituteInPlace {./scripts/helpers/tmux_utils.sh,./scripts/*.sh} \
     --replace 'HELPERS_DIR=' 'HELPERS_DIR="${placeholder "out"}/share/scripts/helpers" # '
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/{share,bin}

    cp -r ./{bindings,scripts} $out/share

    for p in ./bin/*_plugins ; do
      f=$(basename $p)
      mv "./bin/$f" "./bin/tpm_$f";
    done

    cp {./bin/*,./tpm} $out/bin

    chmod +x $out/bin/*

    runHook postInstall
  '';
}

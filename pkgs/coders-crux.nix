{
  stdenvNoCC,
  unzip,
}:
stdenvNoCC.mkDerivation {
  pname = "coders-crux";
  version = "unknown";

  src = builtins.fetchurl {
    url = "https://dl.dafont.com/dl?f=coders_crux";
    sha256 = "18di40afgih01cd50fd0xfckf0f8qncbgvn9gbd0pmah1vxcixn5";
  };

  dontUnpack = true;

  installPhase = ''
    runHook preInstall

    ${unzip}/bin/unzip $src
    install -Dm644 *.ttf -t $out/share/fonts/truetype

    runHook postInstall
  '';
}

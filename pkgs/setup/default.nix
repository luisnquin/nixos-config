{
  lib,
  stdenv,
}:
stdenv.mkDerivation {
  name = "setup";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = ./setup.sh;
  };

  installPhase = ''
    mkdir -p $out/bin
    cp setup.sh $out/bin/setup
    chmod +x $out/bin/setup
  '';
}

{stdenv}:
stdenv.mkDerivation {
  name = "setup";
  src = ./.;
  installPhase = ''
    mkdir -p $out/bin
    cp setup.sh $out/bin/setup
    chmod +x $out/bin/setup
  '';
}

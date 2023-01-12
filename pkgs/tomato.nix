{
  pkgs,
  stdenv,
  fetchgit,
}:
stdenv.mkDerivation {
  name = "tomato";
  src = fetchgit {
    url = "https://github.com/gabrielzschmitz/Tomato.C";
    rev = "62a05d25a42897e2204bbf4f2e85eedce90398d0";
    sha256 = "1bw8wvbw47i3bbhnbg0nsljasqh61wkpx3xhlj75a67ycd7zihp3";
  };
  installPhase = "mkdir -p $out/bin && cp tomato $out/bin/";
  buildInputs = with pkgs; [gcc gnumake ncurses];
}

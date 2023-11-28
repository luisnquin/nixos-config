{
  fetchFromGitHub,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation rec {
  name = "rofi-todo";
  src = fetchFromGitHub {
    owner = "claudiodangelis";
    repo = "rofi-todo";
    rev = "5114d1dbe3ece003d7be6dbc8c3241418b62f85b";
    sha256 = "1nkaxqsf3kf0rw60c8s6n3qaz5xs51f8dbmjj9nafkgpi5cba1s8";
  };

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin
    mv ${name}.sh ${name}
    chmod +x ${name}
    mv ${name} $out/bin

    runHook postInstall
  '';
}

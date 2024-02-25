{
  python3Packages,
  fetchFromGitHub,
  python3,
  ...
}: let
  emojilib-src = fetchFromGitHub {
    name = "emojilib-source";
    owner = "muan";
    repo = "emojilib";
    rev = "v3.0.11";
    hash = "sha256-8ALzVOJMBHRqa/zHP5QfphRqoZYr8Y7yIkNGgtskL+U=";
  };

  gemoji-src = fetchFromGitHub {
    name = "gemoji-source";
    owner = "github";
    repo = "gemoji";
    rev = "v4.1.0";
    hash = "sha256-vs/ltvNzctK6mlKy+fxeVANfiQqueLBr3OvblyRNGvo=";
  };
in
  with python3Packages;
    buildPythonPackage {
      name = "emoji-fzf";

      src = fetchPypi {
        pname = "emoji-fzf";
        version = "0.8.0";
        hash = "sha256-ITiUZKGcAfLyejzc2/SlCXFqLdUwu8p2g+sbGRkmeuQ=";
      };

      propagatedBuildInputs = [
        (python3.withPackages (ps: with ps; [pip]))
        click
        twine
      ];

      postPatch = ''
        substituteInPlace setup.py \
          --replace "emojilib/dist" "${emojilib-src}/dist" \
          --replace "gemoji/db" "${gemoji-src}/db"
      '';
    }

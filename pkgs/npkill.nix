{
  fetchFromGitHub,
  buildNpmPackage,
  lib,
}:
buildNpmPackage rec {
  pname = "npkill";
  version = "0.10.0";

  src = fetchFromGitHub {
    owner = "voidcosmos";
    repo = pname;
    rev = "v${version}";
    sha256 = "sha256-oi/mZ4mLHhfXiLpoXCZ/8UNGzvUpjMMqX9bYRXXhYMY=";
  };

  npmDepsHash = "sha256-NM5Iuq0ys2lDoajfjetxqziZzEeVRNb748vt67oYj3w=";
  npmPackFlags = ["--ignore-scripts"];

  meta = with lib; {
    description = "List any node_modules package dir in your system and how heavy they are. You can then select which ones you want to erase to free up space";
    homepage = "https://npkill.js.org";
    license = licenses.mit;
    maintainers = with maintainers; [luisnquin];
  };
}

{pkgs ? import <nixpkgs> {}}:
pkgs.appimageTools.wrapType2 rec {
  name = "yaak";
  version = "2024.2.0";

  src = builtins.fetchTarball {
    url = "https://releases.yaak.app/${version}/yaak_${version}_amd64.AppImage.tar.gz";
    sha256 = "0c7pq9xg2kfikx1fh4kxzkr3j4wrnzdyz65qb6xpmi4c5psklwl4";
  };
}

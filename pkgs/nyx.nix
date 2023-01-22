{pkgs ? import <nixpkgs> {}}:
pkgs.stdenv.mkDerivation {
  name = "nyx";
  src = pkgs.fetchFromGitHub {
    owner = "luisnquin";
    repo = "nixos-config";
    rev = "81693bd179a390ccbc1432b475e4f3d7247bde9a";
    sha256 = "1pfasi9asfb8s0aqyz9vnp94v4pm2xr41wkfcikm0vpd2vjfzr8g";
  };

  installPhase = ''
    mkdir -p $out/bin/
    cp $src/.scripts/main.sh $out/bin/
    mv $out/bin/main.sh $out/bin/nyx
    chmod +x $out/bin/nyx
  '';
}

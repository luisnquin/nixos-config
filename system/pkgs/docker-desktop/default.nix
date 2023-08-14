{pkgs ? import <nixpkgs> {}}:
pkgs.stdenv.mkDerivation {
  name = "docker-desktop";

  src = pkgs.fetchurl {
    url = "https://desktop.docker.com/linux/main/amd64/docker-desktop-4.21.1-amd64.deb?utm_source=nixpkgs";
    sha256 = "sha256-0io7DZYc9WwMLAzE4/qLizuY7J8ccsVIQbxh2QoBR3A=";
  };

  builder = ./builder.sh;

  propagatedBuildInputs = with pkgs; [
    docker # /nix/store/*-docker-desktop/usr/local/bin/docker
    qemu

    # Shadowed CLI plugins: *usr/lib/docker/cli-plugins/
    # docker-buildx
    # docker-compose
    # docker-dev
    # docker-extension
    # docker-init
    # docker-sbom
    # docker-scan
    # docker-scout
  ];

  inherit (pkgs) dpkg;
}

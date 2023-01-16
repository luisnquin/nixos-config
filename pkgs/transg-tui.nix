{
  lib,
  pkgs,
  rustPlatform,
  fetchFromGitHub,
}:
rustPlatform.buildRustPackage rec {
  pname = "transg-tui";
  version = "0.0.1";

  src = fetchFromGitHub {
    owner = "PanAeon";
    repo = pname;
    rev = "3d06006a03904713aec0a765bd6cd58fc6c3035c";
    sha256 = "sha256-Q+dEy79UszR25mWqkAskP6xgRnZua3CqHGUYOnTZOo0=";
    fetchSubmodules = true;
  };

  cargoSha256 = "sha256-zwK5QKZ9DZhHKm131iWDJ3xlZOu5OcaXo+Cp12RptKw=";

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    openssl
  ];

  meta = with lib; {
    description = "A transgressive way to manage your transmission torrents in the terminal";
    homepage = "https://github.com/PanAeon/transg-tui";
    license = licenses.mit;
    maintainers = ["PanAeon" "luisnquin"];
  };
}

# Source: https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/data/icons/vimix-cursor-theme/default.nix#L27
{
  fetchFromGitHub,
  stdenvNoCC,
}:
stdenvNoCC.mkDerivation rec {
  pname = "vimix-cursor-theme";
  version = "2020-02-24";

  src = fetchFromGitHub {
    owner = "vinceliuice";
    repo = "Vimix-cursors";
    rev = version;
    hash = "sha256-TfcDer85+UOtDMJVZJQr81dDy4ekjYgEvH1RE1IHMi4=";
  };

  installPhase = ''
    sed -i 's/Vimix Cursors$/Vimix-Cursors/g' dist{,-white}/index.theme

    install -dm 755 $out/share/icons/Vimix-Cursors{,-White}

    cp -dr --no-preserve='ownership' dist/*        $out/share/icons/Vimix-Cursors
    cp -dr --no-preserve='ownership' dist-white/*  $out/share/icons/Vimix-Cursors-White
  '';
}

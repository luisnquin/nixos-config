{
  lib,
  stdenvNoCC,
  fetchFromGitHub,
  gnome-shell,
  jdupes,
  sassc,
}:
# nixpkgs dropped every murrine-dependent theme in bcdf2b8ca (2026-07-29) and
# vimix went with it, along with `gtk_engines` and `gtk-engine-murrine`. Those
# only ever fed the GTK2 assets, which nothing here reads, so the derivation
# is upstream's minus the engine inputs. Only the doder family is built —
# `home/modules/desktop/gtk.nix` asks for `Vimix-light-doder`.
stdenvNoCC.mkDerivation rec {
  pname = "vimix-gtk-themes";
  version = "2025-06-20";

  src = fetchFromGitHub {
    owner = "vinceliuice";
    repo = "vimix-gtk-themes";
    rev = version;
    hash = "sha256-uRm6v+Zag4FO7nFVcHhZjVhOfdOeYBZYQym0IBR8+HU=";
  };

  nativeBuildInputs = [
    gnome-shell # detects the gnome-shell version
    jdupes
    sassc
  ];

  postPatch = ''
    patchShebangs install.sh
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/themes
    name= HOME="$TMPDIR" ./install.sh --theme doder --dest $out/share/themes
    rm $out/share/themes/*/{AUTHORS,LICENSE}
    jdupes --quiet --link-soft --recurse $out/share
    runHook postInstall
  '';

  meta = {
    description = "Flat Material Design theme for GTK based desktop environments";
    homepage = "https://github.com/vinceliuice/vimix-gtk-themes";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.unix;
  };
}

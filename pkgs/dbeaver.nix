{
  copyDesktopItems,
  autoPatchelfHook,
  makeDesktopItem,
  glib-networking,
  makeWrapper,
  stdenvNoCC,
  webkitgtk,
  libXtst,
  jdk21,
  glib,
  gtk3,
  lib,
  ...
}:
stdenvNoCC.mkDerivation rec {
  pname = "dbeaver";
  version = "24.0.0";

  src = builtins.fetchTarball {
    name = "dbeaver-archive-no-jdk";
    url = "https://github.com/dbeaver/dbeaver/releases/download/${version}/dbeaver-ce-24.0.0-linux.gtk.x86_64-nojdk.tar.gz";
    sha256 = "0s4vibz8bkn6r1ylm0jy0gih6i6sbhksvmdpa6x7vvphk0196nk5";
  };

  nativeBuildInputs = [
    copyDesktopItems
    autoPatchelfHook
    makeWrapper
  ];

  installPhase = ''
    mkdir -p $out/{dbeaver,bin}
    cp -r ./* $out/dbeaver/

    autoPatchelf $out/dbeaver/dbeaver

    makeWrapper $out/dbeaver/dbeaver $out/bin/dbeaver \
      --prefix PATH : ${jdk21}/bin \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [glib gtk3 libXtst webkitgtk glib-networking]} \
      --prefix GIO_EXTRA_MODULES : "${glib-networking}/lib/gio/modules" \
      --prefix XDG_DATA_DIRS : "$GSETTINGS_SCHEMAS_PATH"
  '';

  desktopItems = [
    (makeDesktopItem {
      name = "dbeaver";
      exec = "dbeaver";
      icon = "dbeaver";
      desktopName = "dbeaver";
      comment = "SQL Integrated Development Environment";
      genericName = "SQL Integrated Development Environment";
      categories = ["Development"];
    })
  ];
}

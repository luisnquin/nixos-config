{
  makeDesktopItem,
  makeWrapper,
  openjdk,
  stdenv,
}:
stdenv.mkDerivation rec {
  name = "minecraft";
  src = builtins.fetchurl {
    url = "https://skmedix.pl/binaries_/SKlauncher-3.1.2.jar";
    sha256 = "627b807380dab8455cd04ba07cdb5a70a7c6f5d510c64296456f41588b60201a";
  };

  dontUnpack = true;

  nativeBuildInputs = [
    makeWrapper
  ];

  propagatedBuildInputs = [
    openjdk
  ];

  installPhase = ''
    mkdir -p $out/{bin,share/icons/hicolor/256x256/apps,share/java}

    cp $src $out/share/java/minecraft.jar

    makeWrapper ${openjdk}/bin/java $out/bin/${name} \
      --add-flags "-jar $out/share/java/minecraft.jar" \
      --set _JAVA_OPTIONS '-Dawt.useSystemAAFontSettings=on' \
      --set _JAVA_AWT_WM_NONREPARENTING 1

    ln -s ${./minecraft.png} $out/share/icons/hicolor/256x256/apps/${name}.png
    ln -s ${desktopItem}/share/applications $out/share
  '';

  desktopItem = makeDesktopItem {
    name = "Minecraft";
    exec = name;
    icon = name;
    desktopName = "Minecraft";
    genericName = "Minecraft is a game made up of blocks, creatures, and community";
    categories = ["Game" "AdventureGame"];
  };
}

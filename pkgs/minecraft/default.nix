{
  makeDesktopItem,
  makeWrapper,
  openjdk,
  stdenv,
}:
stdenv.mkDerivation rec {
  name = "minecraft";
  src = builtins.fetchurl {
    url = "https://files.launcherfenix.com.ar/prelauncher/v7/LauncherFenix-Minecraft-v7.jar";
    sha256 = "1w80fii2nab4whycl4nj20r1imi2sqd9sag7bcbvcbxjqmkxpcz7";
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

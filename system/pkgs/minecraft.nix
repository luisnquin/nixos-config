{
  stdenv,
  makeWrapper,
  openjdk,
  lib,
}:
stdenv.mkDerivation rec {
  name = "minecraft";
  src = builtins.fetchurl {
    url = "https://github.com/ztgasdf/fenix-english/releases/download/v0.2/LauncherFenix-Minecraft-v7-patched.jar";
    sha256 = "118l794ljyc43rn3px67kb5ryw4s8dcz239w487wmj2jf14sbnc1";
  };

  dontUnpack = true;

  nativeBuildInputs = [
    # copyDesktopItems
    makeWrapper
  ];

  propagatedBuildInputs = [
    openjdk
  ];

  installPhase = ''
    mkdir -pv $out/share/java $out/bin
    cp ${src} $out/share/java/minecraft.jar

    makeWrapper ${openjdk}/bin/java $out/bin/minecraft \
      --add-flags "-jar $out/share/java/minecraft.jar" \
      --set _JAVA_OPTIONS '-Dawt.useSystemAAFontSettings=on' \
      --set _JAVA_AWT_WM_NONREPARENTING 1
  '';

  # postInstall = ''
  #   mkdir -p $out/share/applications
  #   ln -s ${minecraft}/share/applications/* $out/share/applications
  # '';

  #desktopItems = [
  #  (makeDesktopItem {
  #    name = "minecraft";
  #    exec = "minecraft";
  #    desktopName = "Minecraft";
  #    genericName = "Minecraft";
  #    icon = "/etc/nixos/pkgs/icons/minecraft.png";
  #    comment = "Join people all over the world playing Minecraft, one of the most popular videogames around!";
  #    categories = ["Game"];
  #    type = "Application";
  #  })
  #];

  meta = with lib; {
    description = "Unofficial English patch for LauncherFenix";
    homepage = "https://github.com/ztgasdf/fenix-english";
    maintainers = with maintainers; ["${owner}"];
  };
}

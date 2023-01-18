{
  # jre,
  stdenv,
  openjdk,
  makeWrapper,
  makeDesktopItem,
  ...
}: let
  #minecraft = makeDesktopItem rec {
  #  name = "Minecraft";
  #  icon = "OpenJK_Icon_128";
  #  exec = "${openjdk}/bin/java -jar ${file}";
  #  comment = "Unofficial English patch for LauncherFenix";
  #  desktopName = "Minecraft";
  #  genericName = "Minecraft";
  #  categories = ["Game"];
  #};
in
  stdenv.mkDerivation rec {
    name = "minecraft";
    src = builtins.fetchurl {
      url = "https://github.com/ztgasdf/fenix-english/releases/download/v0.2/LauncherFenix-Minecraft-v7-patched.jar";
      sha256 = "118l794ljyc43rn3px67kb5ryw4s8dcz239w487wmj2jf14sbnc1";
    };

    dontUnpack = true;

    nativeBuildInputs = [makeWrapper];

    installPhase = ''
      mkdir -pv $out/share/java $out/bin
      cp ${src} $out/share/java/minecraft.jar

      makeWrapper ${openjdk}/bin/java $out/bin/minecraft \
        --add-flags "-jar $out/share/java/minecraft.jar" \
        --set _JAVA_OPTIONS '-Dawt.useSystemAAFontSettings=on' \
        --set _JAVA_AWT_WM_NONREPARENTING 1
    '';

    #postInstall = ''
    #  ls ${minecraft}
    #  mkdir -p $out/share/applications
    #  ln -s ${minecraft}/share/applications/* $out/share/applications
    #'';
  }
#installPhase = ''
#  cp $src $out/bin
#'';


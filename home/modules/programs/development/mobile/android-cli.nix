{
  pkgs,
  ANDROID_HOME,
  ANDROID_SDK_ROOT,
  ...
}: let
  androidFhsLibs = p:
    (with p; [
      alsa-lib
      dbus
      expat
      fontconfig
      freetype
      libbsd
      libdrm
      libGL
      libpng
      libpulseaudio
      libuuid
      libX11
      libxcb
      libxkbcommon
      libxkbfile
      libXcomposite
      libXcursor
      libXdamage
      libXext
      libXfixes
      libXi
      libXrandr
      libXrender
      libXtst
      mesa-demos
      nspr
      nss_latest
      pciutils
      setxkbmap
      systemd
      xcbutilcursor
      xcbutilimage
      xcbutilkeysyms
      xcbutilrenderutil
      xcbutilwm
      xorg.libICE
      xorg.libSM
      zlib
    ])
    ++ [p.pkgsi686Linux.zlib];

  android-bin = pkgs.runCommand "android-bin-0-unstable" {
    src = pkgs.fetchurl {
      url = "https://dl.google.com/android/cli/latest/linux_x86_64/android";
      hash = "sha256-RwnFbpm+IkZSmfEDMUCsDFQIKL6gdRqg4CSZc40Mok4=";
    };
  } ''install -Dm755 "$src" "$out/bin/android"'';
in
  pkgs.buildFHSEnv {
    name = "android-cli";
    targetPkgs = androidFhsLibs;
    runScript = pkgs.writeShellScript "android-cli-dispatcher" ''
      export ANDROID_HOME="''${ANDROID_HOME:-${ANDROID_HOME}}"
      export ANDROID_SDK_ROOT="''${ANDROID_SDK_ROOT:-${ANDROID_SDK_ROOT}}"
      export ANDROID_EMULATOR_USE_SYSTEM_LIBS=1
      cmd="$1"
      shift
      case "$cmd" in
        android)  exec ${android-bin}/bin/android "$@" ;;
        emulator) exec "$ANDROID_SDK_ROOT/emulator/emulator" "$@" ;;
        *) echo "android-cli: unknown command '$cmd'" >&2; exit 64 ;;
      esac
    '';
    extraInstallCommands = ''
      mv $out/bin/android-cli $out/libexec-android-cli
      mkdir -p $out/bin
      for cmd in android emulator; do
        cat > $out/bin/$cmd <<EOF
      #!${pkgs.runtimeShell}
      exec $out/libexec-android-cli $cmd "\$@"
      EOF
        chmod +x $out/bin/$cmd
      done
    '';
  }

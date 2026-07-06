{
  config,
  pkgs,
  ...
}: let
  ANDROID_HOME = "${config.home.homeDirectory}/.android";
  ANDROID_SDK_ROOT = "${ANDROID_HOME}/sdk";
in {
  imports = [
    ./options.nix
  ];

  android.avds.galaxy-s26-plus = {
    displayName = "Galaxy S26+";
    api = "36";

    systemImage = "google_apis_playstore";
    arch = "x86_64";

    manufacturer = "Samsung";
    deviceName = "Galaxy S26 Plus";

    width = 1080;
    height = 2340;
    density = 425;

    ramSize = 8192;

    skin = {
      enable = true;
      name = "galaxy-s26-plus";
      source = ./skins/Galaxy_S26_Plus;
    };
  };

  home = {
    packages = with pkgs; [
      android-studio
      android-tools
      (import ./android-cli.nix {inherit pkgs ANDROID_HOME ANDROID_SDK_ROOT;})

      scrcpy

      kotlin-language-server
      kotlin-native
      kotlin

      sdkmanager # Accept The License - The CLI
    ];

    file = {
      "${ANDROID_HOME}/platform-tools" = {
        source = config.lib.file.mkOutOfStoreSymlink "${pkgs.android-tools}/bin";
      };

      ".gradle/gradle.properties".text = ''
        org.gradle.jvmargs=-Xmx14g -XX:MaxMetaspaceSize=512m -XX:+HeapDumpOnOutOfMemoryError -Dfile.encoding=UTF-8
        org.gradle.parallel=true
        org.gradle.configureondemand=true
        org.gradle.daemon=false
      '';
    };

    sessionVariables = {
      inherit ANDROID_HOME ANDROID_SDK_ROOT;
    };
  };
}

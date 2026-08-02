{
  config,
  pkgs,
  ...
}: let
  ANDROID_HOME = "${config.home.homeDirectory}/.android/sdk";
  ANDROID_SDK_ROOT = ANDROID_HOME;
in {
  systemd.user.slices.android-emulator = {
    Unit.Description = "Android emulator resource control";
    Slice = {
      IOWriteBandwidthMax = "/ 150M";
      MemoryHigh = "12G";
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

      sdkmanager
    ];

    file = {
      ".android/advancedFeatures.ini" = {
        text = ''
          QuickbootFileBacked = off
        '';
        force = true;
      };

      ".gradle/gradle.properties" = {
        text = ''
          org.gradle.jvmargs=-Xmx14g -XX:MaxMetaspaceSize=512m -XX:+HeapDumpOnOutOfMemoryError -Dfile.encoding=UTF-8
          org.gradle.parallel=true
          org.gradle.configureondemand=true
          org.gradle.daemon=false
        '';
        force = true;
      };
    };

    sessionVariables = {
      inherit ANDROID_HOME ANDROID_SDK_ROOT;
    };
  };
}

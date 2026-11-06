{
  config,
  pkgs,
  ...
}: let
  ANDROID_HOME = "${config.home.homeDirectory}/.android/sdk";
  ANDROID_SDK_ROOT = ANDROID_HOME;
in {
  # `eas build --local` copies the repo into a fresh uuid dir under this base
  # every time; relative paths below it are what lets one build hit another's cache.
  xdg.configFile."ccache/ccache.conf".text = ''
    base_dir = /tmp/${config.home.username}/eas-build-local-nodejs
    hash_dir = false
    max_size = 20G
  '';

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
      ccache

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
          org.gradle.jvmargs=-Xmx4g -XX:MaxMetaspaceSize=1g -XX:+HeapDumpOnOutOfMemoryError -Dfile.encoding=UTF-8
          org.gradle.daemon.idletimeout=3600000
          org.gradle.parallel=true
          org.gradle.caching=true
          org.gradle.daemon=true
        '';
        force = true;
      };
    };

    sessionVariables = {
      inherit ANDROID_HOME ANDROID_SDK_ROOT;
      # RN's app CMake picks ccache up alone; every library's CMake needs these.
      CMAKE_C_COMPILER_LAUNCHER = "ccache";
      CMAKE_CXX_COMPILER_LAUNCHER = "ccache";
    };
  };
}

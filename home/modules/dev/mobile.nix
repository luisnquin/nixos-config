{
  config,
  pkgs,
  ...
}: {
  home = let
    ANDROID_HOME = "${config.home.homeDirectory}/.android";
  in {
    packages = with pkgs; [
      android-studio

      kotlin-language-server
      kotlin-native
      kotlin

      android-tools
    ];

    file."${ANDROID_HOME}/platform-tools" = {
      source = config.lib.file.mkOutOfStoreSymlink "${pkgs.android-tools}/bin";
    };

    sessionPath = [
      "${ANDROID_HOME}/platform-tools"
      "${ANDROID_HOME}/emulator"
    ];

    sessionVariables = {
      inherit ANDROID_HOME;
    };
  };
}

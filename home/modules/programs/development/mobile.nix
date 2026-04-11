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
      android-tools

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

    sessionPath = [
      "${ANDROID_HOME}/platform-tools"
      "${ANDROID_HOME}/emulator"
    ];

    sessionVariables = {
      inherit ANDROID_HOME;
      ANDROID_SDK_ROOT = "${ANDROID_HOME}/sdk";
    };
  };
}

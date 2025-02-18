{
  inputs,
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

      maestro # Mobile UI Automation tool

      kotlin-language-server
      kotlin-native
      kotlin

      # every year this day is important, today is not the exception
      # I wanted to try peepshow but
      #
      # In my imagination you're waiting laying on your side
      # With your hands between your thighs
      #
      # It it's a seven-hour flight or a 45-minute drive
      #
      # In my imagination you're waiting laying on your side
      # With your hands between your thighs
      #
      # Did you know that this is a long-term joke? 'Cause I didn't
      inputs.expo-flake.packages.${system}.eas-cli

      sdkmanager # for me, "Accept The License - The CLI"
    ];

    file = {
      "${ANDROID_HOME}/platform-tools" = {
        source = config.lib.file.mkOutOfStoreSymlink "${pkgs.android-tools}/bin";
      };

      # Oh, but we, we couldn't stay together
      # I knew this wouldn't last forever
      # forever, just more one time than never
      # this is the last string to sever
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
    };
  };
}

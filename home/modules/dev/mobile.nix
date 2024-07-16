{
  config,
  pkgs,
  ...
}: {
  home = {
    packages = with pkgs; [
      android-studio

      kotlin-language-server
      kotlin-native
      kotlin
    ];

    sessionVariables = {
      ANDROID_HOME = "${config.home.homeDirectory}/.android";
    };
  };
}

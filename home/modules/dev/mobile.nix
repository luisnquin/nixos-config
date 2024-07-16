{pkgs, ...}: {
  home.packages = with pkgs; [
    android-studio

    kotlin-language-server
    kotlin-native
    kotlin
  ];
}

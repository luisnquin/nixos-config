{pkgs, ...}: {
  home.packages = with pkgs; [
    kotlin-language-server
    kotlin-native
    kotlin
  ];
}

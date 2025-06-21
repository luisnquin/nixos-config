{pkgs, ...}: {
  home.packages = with pkgs; [
    google-chat-linux
    telegram-desktop
    slack
  ];
}

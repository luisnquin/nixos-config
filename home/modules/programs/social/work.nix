{pkgs, ...}: {
  home.packages = with pkgs; [
    google-chat-linux
    slack
  ];
}

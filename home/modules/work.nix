{pkgs, ...}: {
  home.packages = with pkgs; [
    mattermost-desktop
    zoom-us
    slack
  ];
}

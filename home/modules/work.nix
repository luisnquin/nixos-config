{pkgs, ...}: {
  home.packages = with pkgs; [
    zoom-us
    slack
  ];
}

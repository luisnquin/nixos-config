{pkgs, ...}: {
  home.packages = with pkgs; [
    tealdeer
  ];
}

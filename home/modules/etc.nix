{pkgs, ...}: {
  home.packages = with pkgs; [
    translate-shell
    tealdeer
  ];
}

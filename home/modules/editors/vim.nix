{pkgs, ...}: {
  home.packages = with pkgs; [
    vim
  ];
}

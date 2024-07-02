{pkgs, ...}: {
  home.packages = with pkgs; [
    lego
  ];
}

{pkgs, ...}: {
  programs.jujutsu = {
    enable = true;
    package = pkgs.jujutsu;
  };
}

{pkgs, ...}: {
  programs.lazygit = {
    enable = true;
    package = pkgs.lazygit;
  };

  xdg.configFile."lazygit/config.yml".source = builtins.path {
    name = "lazygit-config.yml";
    path = ./dots/lazygit/config.yml;
  };
}

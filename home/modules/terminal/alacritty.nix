{pkgs, ...}: {
  programs.alacritty = {
    enable = true;
    package = pkgs.alacritty;
  };

  xdg.configFile = {
    "alacritty.toml".text = builtins.readFile ./dots/alacritty.toml;
  };
}

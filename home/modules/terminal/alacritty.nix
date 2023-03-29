{
  config,
  pkgs,
  ...
}: {
  programs.alacritty = {
    enable = true;
    package = pkgs.alacritty;
  };

  xdg.configFile = {
    "alacritty.yml".text = builtins.readFile ../../dots/alacritty.yml;
  };
}

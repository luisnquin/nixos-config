{pkgs, ...}: {
  home.packages = [pkgs.kibi];

  xdg.configFile = {
    "kibi/config.ini".text = builtins.readFile ../../dots/kibi/config.ini;
  };
}

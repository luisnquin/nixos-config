{pkgs, ...}: {
  home.packages = with pkgs; [
    btop
  ];

  xdg.configFile = {
    "btop/btop.conf".text = let
      cfgFile = builtins.path {
        name = "btop-config";
        path = ./btop.conf;
      };
    in
      builtins.readFile cfgFile;

    "btop/themes/custom.theme".text = let
      themeFile = builtins.path {
        name = "btop-theme";
        path = ./themes/custom.theme;
      };
    in
      builtins.readFile themeFile;
  };
}

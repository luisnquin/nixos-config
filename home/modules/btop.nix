{config, ...}: {
  xdg.configFile = {
    "btop/btop.conf".text = builtins.readFile ../dots/btop/btop.conf;
    "btop/themes/custom.theme".text = builtins.readFile ../dots/btop/themes/custom.theme;
  };
}

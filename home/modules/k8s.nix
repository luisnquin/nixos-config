{config, ...}: {
  xdg.configFile = {
    "k9s/views.yml".text = builtins.readFile ../dots/k9s/views.yml;
    "k9s/skin.yml".text = builtins.readFile ../dots/k9s/skin.yml;
  };
}

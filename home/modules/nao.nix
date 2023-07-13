{...}: {
  xdg.configFile = {
    "nao/config.yml".text = builtins.readFile ../dots/nao/config.yml;
  };
}

{
  xdg.configFile = {
    "lazydocker/config.yml".text = builtins.readFile ../dots/lazydocker/config.yml;
  };
}

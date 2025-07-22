{pkgs, ...}: {
  home.packages = with pkgs; [
    macchina
  ];

  xdg.configFile = {
    "macchina/macchina.toml".text = builtins.readFile ./macchina.toml;
    "macchina/assets/v.ascii".text = builtins.readFile ./v.ascii;
    "macchina/themes/yttrium.toml".text = builtins.readFile ./themes/yttrium.toml;
  };
}

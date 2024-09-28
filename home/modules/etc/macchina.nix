{pkgs, ...}: {
  home.packages = with pkgs; [
    macchina
  ];

  xdg.configFile = {
    "macchina/macchina.toml".text = builtins.readFile ../../dots/macchina/macchina.toml;
    "macchina/assets/v.ascii".text = builtins.readFile ../../dots/macchina/assets/v.ascii;
    "macchina/themes/yttrium.toml".text = builtins.readFile ../../dots/macchina/themes/yttrium.toml;
  };
}

{
  inputs,
  system,
  pkgs,
  ...
}: {
  programs.rofi = {
    enable = true;
    plugins = with pkgs; [
      rofi-calc
    ];
  };

  home.packages = [
    inputs.rofi-network-manager.defaultPackage.${system}
  ];

  xdg.configFile = {
    "rofi/rofi-network-manager.conf".text = builtins.readFile ./../../dots/rofi/rofi-network-manager.conf;
    "rofi/rofi-network-manager.rasi".text = builtins.readFile ./../../dots/rofi/rofi-network-manager.rasi;
    "rofi/config.rasi".text = builtins.readFile ./../../dots/rofi/config.rasi;
  };
}

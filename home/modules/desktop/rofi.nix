{
  rofi-network-manager,
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
    rofi-network-manager
  ];

  xdg.configFile = {
    "rofi/rofi-network-manager.conf".text = builtins.readFile ./../../dots/rofi/rofi-network-manager.conf;
    "rofi/rofi-network-manager.rasi".text = builtins.readFile ./../../dots/rofi/rofi-network-manager.rasi;
    "rofi/config.rasi".text = builtins.readFile ./../../dots/rofi/config.rasi;
  };
}

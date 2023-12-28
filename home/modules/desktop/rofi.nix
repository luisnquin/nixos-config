{
  rofi-network-manager,
  pkgs,
  ...
}: {
  programs = {
    rofi = {
      enable = true;
      plugins = with pkgs; [
        rofi-calc
      ];
    };

    rofi-radio = {
      enable = true;
      broadcasters = [
        {
          name = "More than ever";
          url = "https://youtube.com/playlist?list=PL_qr1c1sjwPmIC5OLHmIeq1D6lB_xUPny";
          shuffle = true;
        }
        {
          name = "Night City Radio";
          url = "https://youtube.com/playlist?list=PLUh-lR3snT_rcET-QS9tUJaCzEcjuXyyD";
          shuffle = true;
        }
        {
          name = "Box Lofi";
          url = "http://stream.zeno.fm/f3wvbbqmdg8uv";
        }
      ];
    };
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

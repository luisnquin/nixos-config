{pkgs, ...}: {
  programs.rofi = {
    enable = true;
    plugins = with pkgs; [
      rofi-calc
    ];
  };

  xdg.configFile."rofi/config.rasi".text = builtins.readFile ./../../../dots/rofi/config.rasi;
}

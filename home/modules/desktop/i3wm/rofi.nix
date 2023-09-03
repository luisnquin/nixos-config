{pkgs, ...}: {
  programs.rofi = {
    enable = true;

    plugins = with pkgs; [
      rofi-calc
    ];

    theme = "Arc-Dark";
    location = "center";
    terminal = "${pkgs.alacritty}/bin/alacritty";
  };
}

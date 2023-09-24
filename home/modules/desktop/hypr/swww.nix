{pkgs, ...}: {
  home.packages = with pkgs; [
    swww
  ];

  xdg.configFile."hypr/hyprland.conf".text = ''
    exec-once = ${pkgs.swww}/bin/swww init
    exec = ${pkgs.swww}/bin/swww img ${./../../../dots/background-image.png}
  '';
}

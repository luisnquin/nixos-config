{pkgs, ...}: {
  home.packages = with pkgs; [
    wl-clipboard
    cliphist
  ];

  xdg.configFile."hypr/hyprland.conf".text = ''
    exec-once = ${pkgs.wl-clipboard}/bin/wl-paste --watch ${pkgs.cliphist}/bin/cliphist store
  '';
}

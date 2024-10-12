{pkgs, ...}: {
  always = [
    "pkill waybar & sleep 0.1 && waybar"
  ];

  once = [
    "${pkgs.wl-clipboard}/bin/wl-paste --watch ${pkgs.cliphist}/bin/cliphist store -max-items 200"
    "${pkgs.lib.getExe pkgs.xwaylandvideobridge}"
    "dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP"
  ];
}

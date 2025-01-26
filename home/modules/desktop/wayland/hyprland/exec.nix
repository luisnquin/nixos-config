{pkgs, ...}: {
  always = [
    "pkill waybar & sleep 0.1 && waybar"
  ];

  once = [
    "${pkgs.wl-clipboard}/bin/wl-paste --watch ${pkgs.cliphist}/bin/cliphist store -max-items 200"
    "dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP"
    "[workspace 2 silent] ghostty"
    ''${pkgs.extra.hyprdrop}/bin/hyprdrop -i ghostty.hyprdrop 'ghostty --class=ghostty.hyprdrop'; while [ -z "$ADDRESS" ] ; do ADDRESS=$(hyprctl clients -j | jq -r '.[] | select(.class == "ghostty.hyprdrop") | .address'); done; hyprctl dispatch movetoworkspacesilent "10",address:"$ADDRESS"''
  ];
}

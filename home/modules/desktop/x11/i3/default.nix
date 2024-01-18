{
  # https://github.com/Th0rgal/horus-nix-home/blob/master/configs/i3.nix
  # https://github.com/Th0rgal/horus-nix-home/blob/master/configs/compton.nix
  # https://www.reddit.com/r/unixporn/comments/fltmar/i3gaps_nixos_arch_my_incredible_nixos_desktop/
  # https://i3wm.org/docs/userguide.html#configuring
  # https://www.google.com/search?q=i3+enable+transparency&oq=i3+enable+&aqs=chrome.4.69i57j0i19i512l3j0i19i22i30l6.6319j0j1&sourceid=chrome&ie=UTF-8
  # https://github.com/vivien/i3blocks#example
  # https://www.reddit.com/r/NixOS/comments/vdlq9e/how_to_use_window_managers_in_nixos/?rdt=38930
  # https://github.com/i3-wsman/i3-wsman
  # https://github.com/davatorium/rofi
  imports = [
    ./i3status-rust.nix
    ./greenclip.nix
    ./picom.nix
    ./i3.nix

    ../../cursor.nix
    ../../dunst.nix
    ../../rofi.nix
    ../../gtk.nix
  ];
}

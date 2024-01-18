{host, ...}: {
  imports = [
    {
      "hyprland" = ./wayland/hyprland;
      "plasma" = ./x11/plasma;
      "i3" = ./x11/i3;
    }
    .${host.desktop}
  ];

  xsession.enable = true;
}

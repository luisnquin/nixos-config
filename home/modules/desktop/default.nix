{host, ...}: {
  imports = [
    {
      "plasma" = ./plasma;
      "hyprland" = ./hyprland;
      "i3" = ./i3;
    }
    .${host.desktop}
  ];

  xsession.enable = true;
}

{ghostty, ...}: {
  home.packages = [
    ghostty
  ];

  xdg.configFile = {
    "ghostty/config".source = ./config;
  };
}

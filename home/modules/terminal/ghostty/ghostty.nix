{
  inputs,
  system,
  ...
}: {
  home.packages = [
    inputs.ghostty.packages.${system}.default
  ];

  xdg.configFile = {
    "ghostty/config".source = ./config;
  };
}

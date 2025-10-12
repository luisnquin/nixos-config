{pkgs, ...}: {
  home.packages = [
    pkgs.betaflight-configurator
    pkgs.express-lrs-configurator
  ];
}

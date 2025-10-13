{pkgs, ...}: {
  home.packages = with pkgs;[
    betaflight-configurator
    express-lrs-configurator
    mission-planner
  ];
}

{pkgs, ...}: {
  home.packages = with pkgs; [
    betaflight-configurator
    express-lrs-configurator
    inav-configurator
    mission-planner
    qgroundcontrol
  ];
}

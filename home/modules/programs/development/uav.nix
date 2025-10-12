{pkgs, ...}: {
  home.packages = [
    pkgs.betaflight-configurator
  ];

  services.flatpak.packages = [
    {
      appId = "org.expresslrs.ExpressLRSConfigurator";
      origin = "flathub";
    }
  ];
}

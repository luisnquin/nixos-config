{pkgs, ...}: let
  betaflight-configurator = pkgs.stdenv.mkDerivation rec {
    pname = "betaflight-configurator-app";
    version = "unreleased";

    dontUnpack = true;
    nativeBuildInputs = [pkgs.copyDesktopItems];

    desktopItems = [
      (pkgs.makeDesktopItem {
        name = pname;
        desktopName = "Betaflight Configurator";
        comment = "Launch Betaflight Configurator in Chromium";
        exec = pname;
        icon = "chromium"; # TODO: use official icon
        categories = ["Network" "Utility"];
        startupNotify = true;
      })
    ];

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin

      # The executable wrapper
      cat > $out/bin/${pname} <<EOF
      #!/bin/sh
      exec ${pkgs.lib.getExe pkgs.chromium} --no-first-run --app=https://app.betaflight.com "\$@"
      EOF
      chmod +x $out/bin/${pname}

      runHook postInstall
    '';

    meta.description = "A wrapper to open Betaflight Configurator web app in Chromium";
  };
in {
  home.packages = [
    betaflight-configurator
    pkgs.express-lrs-configurator
    pkgs.inav-configurator
    pkgs.mission-planner
    pkgs.qgroundcontrol
  ];
}

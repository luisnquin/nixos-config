{pkgs, ...}: let
  mkChromeApp = {
    pname,
    appUrl,
    desktopName,
    description,
  }:
    pkgs.stdenv.mkDerivation rec {
      inherit pname;
      version = "unreleased";

      dontUnpack = true;
      nativeBuildInputs = [pkgs.copyDesktopItems];

      desktopItems = [
        (pkgs.makeDesktopItem {
          name = pname;
          inherit desktopName;
          comment = description;
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
        exec ${pkgs.lib.getExe pkgs.chromium} --no-first-run --app=${appUrl} "\$@"
        EOF
        chmod +x $out/bin/${pname}

        runHook postInstall
      '';

      meta = {inherit description;};
    };
in {
  home.packages = [
    (mkChromeApp {
      pname = "betaflight-configurator-app";
      appUrl = "https://app.betaflight.com";
      desktopName = "Betaflight Configurator";
      description = "A wrapper to open Betaflight Configurator web app in Chromium";
    })
    (mkChromeApp {
      pname = "esc-configurator-app";
      appUrl = "https://esc-configurator.com/";
      desktopName = "ESC Configurator";
      description = "A wrapper to open EmuFlight Configurator web app in Chromium";
    })
    pkgs.express-lrs-configurator
    pkgs.inav-configurator
    pkgs.mission-planner
    pkgs.qgroundcontrol
  ];
}

{
  fetchurl,
  chromium,
  lib,
  makeDesktopItem,
  runtimeShell,
  symlinkJoin,
  writeScriptBin,
}: let
  name = "netflix";

  desktopItem = makeDesktopItem {
    inherit name;
    # Executing by name as opposed to store path is conventional and prevents
    # copies of the desktop file from bitrotting too much.
    # (e.g. a copy in ~/.config/autostart, you lazy lazy bastard ;) )
    exec = name;
    icon = fetchurl {
      name = "netflix-icon-2016.png";
      url = "https://assets.nflxext.com/us/ffe/siteui/common/icons/nficon2016.png";
      sha256 = "sha256-c0H3uLCuPA2krqVZ78MfC1PZ253SkWZP3PfWGP2V7Yo=";
      meta.license = lib.licenses.unfree;
    };
    desktopName = "Netflix";
    genericName = "A video streaming service providing films and exclusive TV series";
    categories = ["TV" "AudioVideo" "Network"];
    startupNotify = true;
  };

  script = writeScriptBin name ''
    #!${runtimeShell}
    exec ${chromium}/bin/chromium \
      --app=https://netflix.com \
      --no-first-run \
      --no-default-browser-check \
      --no-crash-upload \
      "$@"
  '';
in
  symlinkJoin {
    inherit name;
    paths = [script desktopItem];
  }

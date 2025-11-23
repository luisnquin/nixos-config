{
  config,
  pkgs,
  lib,
  ...
}: let
  cohesionAppId = "io.github.brunofin.Cohesion";
in {
  home.packages = with pkgs; [
    google-chat-linux
    zoom-us
  ];

  services.flatpak.packages = [
    {
      appId = cohesionAppId;
      origin = "flathub";
    }
  ];

  xdg.desktopEntries.notion = {
    name = "Notion";
    exec = "${lib.getExe pkgs.dex} ${config.home.homeDirectory}/.local/share/flatpak/exports/share/applications/${cohesionAppId}.desktop";
    mimeType = ["x-scheme-handler/notion"];
  };
}

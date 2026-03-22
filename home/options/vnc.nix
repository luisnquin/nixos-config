{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.vnc;

  mkConfigFile = name: entry: {
    name = "vnc/${name}.vnc";
    value.text = ''
      host=${entry.host}
      port=${toString entry.port}

      UseLocalCursor=0
      PreferredEncoding=${entry.encoding}
      FullScreen=${
        if entry.fullScreen
        then "1"
        else "0"
      }
      Scaling=${entry.scaling}
      Quality=${entry.quality}
    '';
  };
in {
  options.programs.vnc = {
    enable = mkEnableOption "VNC declarative entries";

    entries = mkOption {
      description = "Declarative configurations for VNC connections.";
      type = types.attrsOf (types.submodule ({...}: {
        options = {
          host = mkOption {
            type = types.str;
            description = "Host address (IP or domain) of the VNC server.";
          };

          port = mkOption {
            type = types.int;
            default = 5900;
            description = "VNC connection port.";
          };

          encoding = mkOption {
            type = types.str;
            default = "hextile";
            description = "Preferred encoding.";
          };

          fullScreen = mkOption {
            type = types.bool;
            default = false;
            description = "Start in full screen mode.";
          };

          scaling = mkOption {
            type = types.str;
            default = "FitAuto";
            description = "Window scaling mode.";
          };

          quality = mkOption {
            type = types.str;
            default = "High";
            description = "Image quality of the connection.";
          };
        };
      }));
      default = {};
    };
  };

  config = mkIf cfg.enable {
    xdg.configFile = mapAttrs' mkConfigFile cfg.entries;

    xdg.desktopEntries =
      mapAttrs (name: entry: {
        name = "${name} (VNC)";
        genericName = "VNC Client";
        exec = "${pkgs.realvnc-vnc-viewer}/bin/vncviewer -config ${config.xdg.configHome}/vnc/${name}.vnc";
        icon = "network-server";
        terminal = false;
        categories = ["Network" "RemoteAccess"];
        type = "Application";
      })
      cfg.entries;

    home.packages = [pkgs.realvnc-vnc-viewer];
  };
}

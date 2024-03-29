{
  config,
  pkgs,
  libx,
  lib,
  ...
}:
with lib; {
  options = {
    services.rustypaste = {
      enable = mkEnableOption "rustypaste";
      settings = mkOption {
        type = types.attrsOf types.anything;
      };
    };
  };

  config = let
    cfg = config.services.rustypaste;
  in
    mkIf cfg.enable {
      systemd.services.rustypaste-server = let
        configPath = builtins.toFile "rustypaste-config.toml" (libx.serde.toTOML (cfg.settings));
      in {
        enable = true;
        description = "A minimal file upload/pastebin service.";
        serviceConfig = {
          Type = "simple";
          ExecStart = "${getExe pkgs.bash} -c 'CONFIG=${configPath} ${lib.getExe pkgs.rustypaste}'";
        };

        wantedBy = ["multi-user.target"];
      };
    };
}

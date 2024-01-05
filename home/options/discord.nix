{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.discord;
in {
  # https://github.com/NotAShelf/nyx/blob/main/homes/notashelf/graphical/apps/discord/default.nix
  options = {
    programs.discord = {
      enable = mkEnableOption "discord";
      withOpenASAR = mkOption {
        default = false;
        type = types.bool;
      };
      withVencord = mkOption {
        default = false;
        type = types.bool;
      };
      waylandSupport = mkOption {
        default = false;
        type = types.bool;
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = let
      discord = pkgs.discord.override {
        inherit (cfg) withVencord withOpenASAR;
        nss = pkgs.nss_latest;
      };
    in [
      (
        if cfg.waylandSupport
        then
          (discord.overrideAttrs (old: {
            libPath = old.libPath + ":${pkgs.libglvnd}/lib";
            nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.makeWrapper];
            postFixup = ''
              wrapProgram $out/opt/Discord/Discord --add-flags "\''${NIXOS_OZONE_WL:+\''${WAYLAND_DISPLAY:+--ozone-platform=wayland}}"
            '';
          }))
        else discord
      )
    ];
  };
}

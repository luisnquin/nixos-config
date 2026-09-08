{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.pinentry-gate;
in {
  options.programs.pinentry-gate = {
    enable = lib.mkEnableOption "pinentry-gate, the pinentry that asks on every surface at once";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.pinentry-gate;
      defaultText = lib.literalExpression "pkgs.pinentry-gate";
      description = "The pinentry package.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      description = ''
        Whose prompts these are. Owns the reserved console, and only this
        user's graphical session counts as one to put a window on.
      '';
    };

    vt = lib.mkOption {
      type = lib.types.nullOr lib.types.int;
      default = 10;
      description = ''
        The virtual console the modal switches to when the seat is not on the
        user's graphical session. Kept above logind's automatic gettys, and
        handed to the user by udev so no capability is needed to switch.
        Null disables the console surface.
      '';
    };

    terminal = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      example = lib.literalExpression ''["''${lib.getExe pkgs.ghostty}" "--fullscreen=true" "-e"]'';
      description = ''
        argv prefix that opens a terminal window running the command appended
        to it, used while the user's graphical session is on the seat. Empty
        skips the window and goes straight to the console.
      '';
    };

    timeout = lib.mkOption {
      type = lib.types.int;
      default = 300;
      description = "Seconds a request waits for a decision before it is cancelled.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [cfg.package];

    environment.etc."pinentry-gate/config.json".text = builtins.toJSON {
      inherit (cfg) user vt terminal timeout;
    };

    services.udev.extraRules = lib.mkIf (cfg.vt != null) ''
      KERNEL=="tty${toString cfg.vt}", OWNER="${cfg.user}", MODE="0600"
    '';

    programs.gnupg.agent.pinentryPackage = lib.mkDefault cfg.package;
  };
}

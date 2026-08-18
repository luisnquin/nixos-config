{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.cliplenz;

  inherit (lib) mkEnableOption mkIf mkOption mkPackageOption types;
in {
  options.programs.cliplenz = {
    enable = mkEnableOption "cliplenz, a native clipboard viewer for cliphizt";

    package = mkPackageOption pkgs "cliplenz" {};

    fonts = mkOption {
      type = types.listOf types.package;
      default = [pkgs.cascadia-code pkgs.dejavu_fonts];
      defaultText = lib.literalExpression "[pkgs.cascadia-code pkgs.dejavu_fonts]";
      example = lib.literalExpression ''
        [pkgs.cascadia-code pkgs.dejavu_fonts pkgs.noto-fonts-cjk-sans pkgs.noto-fonts-color-emoji]
      '';
      description = ''
        Fonts cliplenz may load, replacing the system fontconfig rather than
        adding to it. Order matters: the first entry drives the interface font
        (see `defaultFont`), later entries only widen preview coverage. Adding
        CJK or emoji packages buys full coverage; faces are memory-mapped, so
        the cost is paged in on demand rather than at startup.
      '';
    };

    defaultFont = mkOption {
      type = types.str;
      default = "";
      example = "DejaVu Sans Mono";
      description = ''
        Interface font family. Empty derives it from the first entry of `fonts`,
        keeping that list the single source of truth; set a family name when
        that package ships several. Overridden per-invocation by `--font`.
      '';
    };
  };

  config = mkIf cfg.enable {
    home.packages = [(cfg.package.override {inherit (cfg) fonts defaultFont;})];
  };
}

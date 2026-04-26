{
  config,
  inputs,
  pkgs,
  lib,
  ...
}: let
  inherit (lib) mkOption types;

  tomlFormat = pkgs.formats.toml {};
  cfg = config.programs.roborev;

  envVarType = types.oneOf [
    types.str
    types.int
    types.float
    types.bool
  ];

  toEnvString = value:
    if builtins.isBool value
    then lib.boolToString value
    else builtins.toString value;

  wrapperArgs = lib.concatStringsSep " " (
    lib.mapAttrsToList (
      name: value: "--set ${lib.escapeShellArg name} ${lib.escapeShellArg (toEnvString value)}"
    )
    cfg.env
  );

  wrappedPackage = pkgs.symlinkJoin {
    name = "${cfg.package.pname or cfg.package.name}-wrapped";
    paths = [cfg.package];
    nativeBuildInputs = [pkgs.makeWrapper];

    postBuild = lib.optionalString (cfg.env != {}) ''
      wrapProgram "$out/bin/roborev" ${wrapperArgs}
    '';
  };
in {
  options = {
    programs.roborev = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Whether to install and configure Roborev.";
      };

      package = mkOption {
        type = types.package;
        default = inputs.roborev.packages.${pkgs.stdenv.hostPlatform.system}.default;
        description = "Roborev package to install.";
      };

      settings = mkOption {
        type = tomlFormat.type;
        default = {};
        description = "Roborev configuration written to ~/.roborev/config.toml.";
      };

      env = mkOption {
        type = types.attrsOf envVarType;
        default = {};
        example = {
          ROBOREV_NO_COLOR = true;
        };
        description = "Environment variables set on the wrapped Roborev executable.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = builtins.all (
          name: builtins.match "[A-Z_][A-Z0-9_]*" name != null
        ) (builtins.attrNames cfg.env);
        message = "programs.roborev.env attributes must be valid environment variable names.";
      }
    ];

    home.packages = [
      (
        if cfg.env == {}
        then cfg.package
        else wrappedPackage
      )
    ];

    home.file.".roborev/config.toml".source =
      tomlFormat.generate "roborev-config.toml" cfg.settings;
  };
}

{
  lib,
  pkgs,
  config,
  ...
}:
with lib; let
  cfg = config.programs.self;

  sys = {
    run = cmd:
      let
        isAttrSet = builtins.isAttrs cmd;
        workdir = if isAttrSet then cmd.workdir else ".";
        command = if isAttrSet then cmd.cmd else cmd;
      in ''
        (
          cd ${workdir}
          printf "\n\e[38;2;112;112;112m(${workdir})\033[0;32m ${command}\033[0m\n"
          ${command}
        )
      '';

    log = msg: ''
      printf "\n%s\n" "${msg}"
    '';

    requireSudo = ''
      if [ "$(id -u)" -ne 0 ]; then sudo echo -n ""; fi
    '';

    notify = title: body: ''
      if command -v notify-send >/dev/null; then
        notify-send "${title}" "${body}"
      fi
    '';

    cd = path: body: ''
      (
        cd ${path}
        ${body}
      )
    '';

    seq = steps: concatStringsSep "\n" steps;

    compose = cmdName: subNames:
      builtins.concatMap (
        subName:
          if cfg.commands.${cmdName}.subcommands ? ${subName}
          then cfg.commands.${cmdName}.subcommands.${subName}.steps
          else []
      )
      subNames;
  };

  renderSteps = cmd:
    if cmd.steps == []
    then ""
    else concatStringsSep "\n" cmd.steps;

  renderSubcommands = cmd:
    concatStringsSep "\n" (
      mapAttrsToList (subName: subCmd: ''
        ${subName})
          ${renderSteps subCmd}
          ;;
      '')
      cmd.subcommands
    );

  renderCommand = name: cmd: let
    hasSub = cmd.subcommands != {};
    hasSteps = cmd.steps != [];
  in ''
    ${name})
      ${optionalString hasSub ''
      SUBCOMMAND="$1"
      shift || true

      case "$SUBCOMMAND" in
        ${renderSubcommands cmd}

        -h|--help)
          echo "${name} [subcommand]"
          ${concatStringsSep "\n" (mapAttrsToList (s: v: ''
          echo "  ${s} - ${v.description}"
        '')
        cmd.subcommands)}
          ;;

        "")
          ${optionalString hasSteps (renderSteps cmd)}
          ;;

        *)
          echo "Unknown subcommand: $SUBCOMMAND"
          exit 1
          ;;
      esac
    ''}

      ${optionalString (!hasSub && hasSteps) (renderSteps cmd)}
      ;;
  '';

  renderHelp = ''
    echo "${config.networking.hostName} [command] [flags]"
    echo
    echo "Available commands:"
    ${concatStringsSep "\n" (mapAttrsToList (name: cmd: ''
        echo "  ${name}   ${cmd.description}"
      '')
      cfg.commands)}
    echo
    echo "Global flags:"
    echo " -h, --help    Print help information"
  '';

  cliScript = ''
    #!/usr/bin/env bash
    set -e

    COMMAND="$1"
    shift || true

    case "$COMMAND" in
      ${concatStringsSep "\n" (mapAttrsToList renderCommand cfg.commands)}

      -h|--help|"")
        ${renderHelp}
        ;;

      *)
        echo "Unknown command: $COMMAND"
        exit 1
        ;;
    esac
  '';
in {
  options.programs.self = {
    enable = mkEnableOption "Declarative CLI tool";

    commands = mkOption {
      type = types.attrsOf (types.submodule ({...}: {
        options = {
          description = mkOption {
            type = types.str;
            default = "";
          };

          script = mkOption {
            type = types.nullOr types.lines;
            default = null;
          };

          steps = mkOption {
            type = types.listOf types.lines;
            default = [];
          };

          subcommands = mkOption {
            type = types.attrsOf (types.submodule ({...}: {
              options = {
                description = mkOption {
                  type = types.str;
                  default = "";
                };

                script = mkOption {
                  type = types.nullOr types.lines;
                  default = null;
                };

                steps = mkOption {
                  type = types.listOf types.lines;
                  default = [];
                };
              };
            }));
            default = {};
          };
        };
      }));
      default = {};
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [
      (pkgs.writeShellScriptBin "${config.networking.hostName}" cliScript)
    ];

    _module.args.sys = sys;
  };
}

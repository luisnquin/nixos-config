{
  lib,
  pkgs,
  config,
  libx,
  ...
}:
with lib; let
  cfg = config.programs.hostcmd;

  defaultNyxIcon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";

  sys = {
    run = cmd: let
      isAttrSet = builtins.isAttrs cmd;
      workdir =
        if isAttrSet
        then cmd.workdir
        else ".";
      command =
        if isAttrSet
        then cmd.cmd
        else cmd;
    in ''
      (
        trap __nyx_on_int INT
        cd ${workdir}
        printf "\n\e[38;2;112;112;112m(${workdir})\033[0;32m ${command}\033[0m\n"
        ${command}
      ) || exit "$?"
    '';

    log = msg: ''
      printf "\n%s\n" "${msg}"
    '';

    requireSudo = ''
      if [ "$(id -u)" -ne 0 ]; then sudo -v || exit "$?"; fi
    '';

    notify = title: body: ''
      if command -v notify-send >/dev/null; then
        notify-send "${title}" "${body}"
      fi
    '';

    cd = path: body: ''
      (
        trap __nyx_on_int INT
        cd ${path}
        ${body}
      ) || exit "$?"
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

    /*
    Runs steps in a subshell and only notifies from the outermost wrapper. Nested
    wrappers propagate their status so composed commands produce one final notification.

    Status is captured explicitly instead of relying on `set -e`: bash suspends
    errexit inside a subshell used as an `if` condition, so a failing step there
    would not abort the run.
    */
    withNotify = {
      image ? defaultNyxIcon,
      topic ? "nyx",
      successTitle,
      successBody,
      failureTitle,
      failureBody,
      successNtfy ? {},
      failureNtfy ? {},
    }: steps: let
      ntfyHost = config.services.ntfy-sh.settings.base-url or "";
      okNotify = libx.notify.send {
        desktop = {
          inherit image;
          title = successTitle;
          message = successBody;
        };
        ntfy =
          {
            host = ntfyHost;
            inherit topic;
            tags = "white_check_mark";
          }
          // successNtfy;
      };
      errNotify = libx.notify.send {
        desktop = {
          inherit image;
          title = failureTitle;
          message = failureBody;
        };
        ntfy =
          {
            host = ntfyHost;
            inherit topic;
            tags = "x";
            priority = 5;
          }
          // failureNtfy;
      };
      inner = concatStringsSep "\n" steps;
    in [
      ''
        __nyx_depth="''${NYX_NOTIFY_DEPTH:-0}"
        __nyx_status=0

        (
          trap __nyx_on_int INT
          export NYX_NOTIFY_DEPTH="$((__nyx_depth + 1))"
          ${inner}
        ) || __nyx_status=$?

        if [ "$__nyx_status" -ne 0 ]; then
          if [ "$__nyx_depth" -eq 0 ] && [ "$__nyx_status" -ne 130 ] && ! __nyx_interrupted; then
            ${errNotify} || true
          fi
          exit "$__nyx_status"
        fi

        if [ "$__nyx_depth" -eq 0 ]; then
          ${okNotify} || true
        fi
      ''
    ];
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

  interruptNotify = libx.notify.send {
    desktop = {
      image = defaultNyxIcon;
      title = "${config.networking.hostName} command interrupted";
      message = "The operation was stopped with Ctrl-C.";
    };
    ntfy = {
      host = config.services.ntfy-sh.settings.base-url or "";
      topic = "nyx";
      tags = "warning";
    };
  };

  cliScript = ''
    #!/usr/bin/env bash
    set -e

    __nyx_root_pid="$BASHPID"
    __nyx_int_flag="''${TMPDIR:-/tmp}/nyx-interrupt-$$"

    # Subshells reset traps, so every nested level re-arms __nyx_on_int and the
    # flag file is what carries "the user pressed Ctrl-C" back to the root shell.
    __nyx_on_int() {
      if [ ! -e "$__nyx_int_flag" ]; then
        : > "$__nyx_int_flag" 2>/dev/null || true
        printf "\n" >&2
      fi
      exit 130
    }

    __nyx_interrupted() {
      [ -e "$__nyx_int_flag" ]
    }

    notify_on_interrupt() {
      status=$?
      [ "$BASHPID" -eq "$__nyx_root_pid" ] || return 0
      if [ "$status" -eq 130 ] || __nyx_interrupted; then
        ${interruptNotify} || true
      fi
      rm -f "$__nyx_int_flag"
    }

    trap __nyx_on_int INT
    trap notify_on_interrupt EXIT

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
  options.programs.hostcmd = {
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

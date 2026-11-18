{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.tripwire;
  home = lib.removeSuffix "/" config.users.users.${cfg.user}.home;
  paths = lib.escapeShellArgs (map (decoy: "${home}/${decoy}") cfg.decoys);

  asUser = "setpriv --reuid=${cfg.user} --regid=$(id -g ${cfg.user}) --init-groups";

  hook = pkgs.writeShellScript "tripwire-alert" ''
    if [ "$TRIPWIRE_EVENT" = opened ]; then
      body="Frozen. kill -CONT -- $TRIPWIRE_FROZEN resumes it, kill -KILL -- $TRIPWIRE_FROZEN ends it.
    ''${TRIPWIRE_CHAIN:0:400}"
    else
      body="The decoy was deleted, moved or altered. It is being recreated."
    fi

    ${asUser} env DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u ${cfg.user})/bus" \
      notify-send -u critical -a tripwire "Decoy $TRIPWIRE_EVENT: $TRIPWIRE_PATH" "$body"

    ${cfg.extraAlert}
  '';
in {
  options.services.tripwire = {
    enable = lib.mkEnableOption "tripwire, which denies every open of a decoy file and freezes the process that tried";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.tripwire;
      defaultText = lib.literalExpression "pkgs.tripwire";
      description = "The tripwire package.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      description = "Whose home holds the decoys, and whose session gets the alert.";
    };

    decoys = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      example = [".git-credentials"];
      description = ''
        Paths under the user's home that nothing legitimate reads. A missing
        one is created with random bytes; an existing file is never replaced.
        No process can read them, so the contents never matter.
      '';
    };

    extraAlert = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = ''
        Shell run as root after the desktop notification, with TRIPWIRE_EVENT
        (opened or removed), TRIPWIRE_PATH, TRIPWIRE_FROZEN (the kill target)
        and TRIPWIRE_CHAIN (the opener and its ancestors) in the environment.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.tripwire = {
      description = "Freeze whatever opens a decoy file";
      wantedBy = ["multi-user.target"];
      unitConfig = {
        RequiresMountsFor = home;
        StartLimitIntervalSec = 0;
      };
      path = with pkgs; [coreutils util-linux libnotify];

      preStart = ''
        for decoy in ${paths}; do
          [ -e "$decoy" ] || ${asUser} ${pkgs.runtimeShell} -c \
            'umask 077 && mkdir -p "$(dirname "$1")" && head -c 1024 /dev/urandom > "$1"' sh "$decoy"
        done
      '';

      serviceConfig = {
        ExecStart = "${lib.getExe cfg.package} --hook ${hook} ${paths}";
        Restart = "always";
        RestartSec = 1;
      };
    };
  };
}

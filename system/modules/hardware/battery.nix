{
  pkgs,
  host,
  user,
  ...
}: let
  batteryPath = "/sys/class/power_supply/BAT1";
  thresholdFile = "${batteryPath}/charge_control_end_threshold";

  safeLimit = builtins.toString host.batteryThreshold;
  writeThreshold = "${pkgs.coreutils}/bin/tee ${thresholdFile}";

  batlimit = pkgs.writeShellApplication {
    name = "batlimit";
    runtimeInputs = [pkgs.coreutils];
    text = ''
      current="$(cat ${thresholdFile})"
      target="''${1-}"

      if [ -z "$target" ]; then
        if [ "$current" = "${safeLimit}" ]; then
          target="100"
        else
          target="${safeLimit}"
        fi
      fi

      case "$target" in
        "" | *[!0-9]*)
          echo "batlimit: expected a number, got '$target'" >&2
          exit 1
          ;;
      esac

      if [ "$target" -lt 1 ] || [ "$target" -gt 100 ]; then
        echo "batlimit: limit must be between 1 and 100" >&2
        exit 1
      fi

      printf '%s\n' "$target" | sudo -n ${writeThreshold} >/dev/null
      printf 'charge limit: %s%% -> %s%%\n' "$current" "$target"
    '';
  };
in {
  environment.systemPackages = [batlimit];

  security.sudo.extraRules = [
    {
      users = [user.alias];
      commands = [
        {
          command = writeThreshold;
          options = ["NOPASSWD"];
        }
      ];
    }
  ];

  programs.zsh.interactiveShellInit = ''
    batty() {
      printf '%s%% (%s), limit %s%%\n' \
        "$(cat ${batteryPath}/capacity)" \
        "$(cat ${batteryPath}/status)" \
        "$(cat ${thresholdFile})"
    }
  '';

  services.logind.settings.Login.HandleLidSwitch = "ignore";
  services.logind.settings.Login.HandleLidSwitchExternalPower = "ignore";

  systemd.services.battery-charge-threshold = {
    enable = host.isLaptop;
    description = "Set the battery charge threshold";

    serviceConfig = {
      Type = "oneshot";
      ExecStart = pkgs.writeShellScript "battery-charge-threshold" ''
        echo ${safeLimit} >${thresholdFile}
      '';
    };

    after = ["suspend.target" "hibernate.target" "hybrid-sleep.target"];
    wantedBy = ["multi-user.target" "suspend.target" "hibernate.target" "hybrid-sleep.target"];
  };
}

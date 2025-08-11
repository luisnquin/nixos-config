{
  inputs,
  system,
  pkgs,
  host,
  user,
  lib,
  ...
}: let
  powerSupplyBatteryPath = "/sys/class/power_supply/BAT1";
in {
  environment.systemPackages = let
    inherit (inputs.nix-scripts.packages.${system}) batlimit;
  in [
    (batlimit.override {inherit powerSupplyBatteryPath;})
  ];

  security.sudo.extraRules = [
    {
      users = [user.alias];
      commands = [
        {
          command = "/run/current-system/sw/bin/batlimit";
          options = ["NOPASSWD"];
        }
      ];
    }
  ];

  programs.zsh.interactiveShellInit = ''
    batty() {
      cat ${powerSupplyBatteryPath}/capacity
    }
  '';

  services = {
    watt = lib.mkIf host.isLaptop {
      enable = true;
      settings = {
        charger = {
          governor = "performance";
          turbo = "auto";
          epp = "performance";
          epb = "balance_performance";
          platform_profile = "performance";
          min_freq_mhz = 800;
          max_freq_mhz = 3500;
        };

        battery = {
          governor = "powersave";
          turbo = "auto";
          epp = "power";
          epb = "balance_power";
          platform_profile = "low-power";
          min_freq_mhz = 800;
          max_freq_mhz = 2500;
        };

        daemon = {
          poll_interval_sec = 4;
          adaptive_interval = true;
          min_poll_interval_sec = 1;
          max_poll_interval_sec = 30;
          throttle_on_battery = true;
        };
      };
    };

    logind.lidSwitchExternalPower = "ignore"; # prevent lid switch from triggering a suspend
  };

  systemd.services.battery-charge-threshold = {
    enable = host.isLaptop;
    description = "Set the battery charge threshold";
    serviceConfig = let
      offset = 1;

      batteryThreshold = builtins.toString (host.batteryThreshold + offset);
      batteryChargeThresholdPath = "${powerSupplyBatteryPath}/charge_control_end_threshold";
    in {
      Type = "oneshot";
      ExecStart = ''${pkgs.bash}/bin/bash -c "echo ${batteryThreshold} > ${batteryChargeThresholdPath}"'';
    };
    wantedBy = ["multi-user.target"];
  };
}

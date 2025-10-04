{
  inputs,
  system,
  pkgs,
  host,
  user,
  ...
}: let
  powerSupplyBatteryPath = "/sys/class/power_supply/BAT1";

  offset = 1;
  safeChargeLimit = builtins.toString (host.batteryThreshold + offset);
in {
  environment.systemPackages = let
    inherit (inputs.nix-scripts.packages.${system}) batlimit;
  in [
    (batlimit.override {
      inherit powerSupplyBatteryPath safeChargeLimit;
    })
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

  services.logind.settings.Login.HandleLidSwitchExternalPower = "ignore"; # prevent lid switch from triggering a suspend

  systemd.services.battery-charge-threshold = {
    enable = host.isLaptop;
    description = "Set the battery charge threshold";
    serviceConfig = let
      batteryChargeThresholdPath = "${powerSupplyBatteryPath}/charge_control_end_threshold";
    in {
      Type = "oneshot";
      ExecStart = ''${pkgs.bash}/bin/bash -c "echo ${safeChargeLimit} > ${batteryChargeThresholdPath}"'';
    };
    wantedBy = ["multi-user.target"];
  };
}

{
  nixosConfig,
  pkgs,
  lib,
  ...
}: {
  systemd.user.services = lib.mkIf nixosConfig.hardware.bluetooth.enable {
    bluetooth-mpris-proxy = {
      Unit = {
        Description = "Bluetooth mpris proxy";
      };

      Service = {
        Type = "simple";
        ExecStart = "${pkgs.bluez}/bin/mpris-proxy";
        Restart = "on-failure";
      };

      Install = {
        # After = ["network.target" "sound.target"];
        WantedBy = ["default.target"];
      };
    };
  };
}

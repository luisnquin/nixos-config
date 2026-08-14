{
  lib,
  pkgs,
  ...
}: {
  xdg.configFile."wayvnc/config".text = ''
    address=0.0.0.0
    port=5900
  '';

  systemd.user.services.wayvnc = {
    Unit = {
      Description = "Wayland VNC server";
      PartOf = ["graphical-session.target"];
    };

    Service = {
      ExecStart = "${lib.getExe pkgs.wayvnc} --max-fps 24";
      Restart = "on-failure";
      RestartSec = 2;
    };

    Install.WantedBy = ["graphical-session.target"];
  };
}

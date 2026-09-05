{
  lib,
  pkgs,
  ...
}: {
  xdg.configFile."wayvnc/config".text = ''
    address=127.0.0.1
    port=5900
  '';

  systemd.user.services.wayvnc = {
    Unit = {
      Description = "Wayland VNC server";
      PartOf = ["graphical-session.target"];
    };

    Service = {
      # wayvnc grabs whichever wl_output the compositor advertises first, which
      # is the external one here; pin it to the built-in panel.
      ExecStart = "${lib.getExe pkgs.wayvnc} --max-fps 24 --output=eDP-1";
      Restart = "on-failure";
      RestartSec = 2;
    };

    Install.WantedBy = ["graphical-session.target"];
  };
}

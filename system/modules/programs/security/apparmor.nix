{pkgs, ...}: {
  security.apparmor = {
    enable = true;
    packages = [pkgs.apparmor-profiles];

    # Keep first rollout stable. New profiles apply to restarted processes.
    killUnconfinedConfinables = false;
  };

  services.dbus.apparmor = "enabled";

  environment.shellAliases = {
    aa-denials = "journalctl -b --grep='apparmor=\"DENIED\"'";
    aa-logprof-today = "journalctl -b --since today --grep audit: | sudo aa-logprof";
  };
}

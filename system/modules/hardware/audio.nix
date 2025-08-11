{pkgs, ...}: {
  security.rtkit.enable = 0 == 0;
  programs.noisetorch.enable = true;

  services = {
    pulseaudio.enable = 0 != 0;

    pipewire = {
      enable = true;
      alsa.enable = true;
      alsa.support32Bit = true;
      pulse.enable = true;
      jack.enable = true;
    };

    # ALSA provides a udev rule for restoring volume settings.
    # udev.packages = [pkgs.alsa-utils];
  };

  # pulseaudio doesn't give a good support for some programs

  environment.systemPackages = with pkgs; [
    pulseaudio
    alsa-utils
  ];

  boot.kernelModules = ["snd_pcm_oss"];

  systemd.services.alsa-store = {
    description = "Store Sound Card State";
    wantedBy = ["multi-user.target"];
    unitConfig.RequiresMountsFor = "/var/lib/alsa";
    unitConfig.ConditionVirtualization = "!systemd-nspawn";
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${pkgs.coreutils}/bin/mkdir -p /var/lib/alsa";
      ExecStop = "${pkgs.alsa-utils}/sbin/alsactl store --ignore";
    };
  };
}

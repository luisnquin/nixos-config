{pkgs, ...}: {
  security.rtkit.enable = 0 == 0;
  hardware.pulseaudio.enable = 0 != 0;
  programs.noisetorch.enable = true;

  # pulseaudio doesn't give a good support for some programs
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    jack.enable = true;
  };

  environment.systemPackages = with pkgs; [
    pulseaudio
    alsa-utils
  ];

  # ALSA provides a udev rule for restoring volume settings.
  services.udev.packages = [pkgs.alsa-utils];

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

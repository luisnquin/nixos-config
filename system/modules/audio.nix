{pkgs, ...}: {
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
  ];

  programs.noisetorch.enable = true;
  sound.enable = true;
}

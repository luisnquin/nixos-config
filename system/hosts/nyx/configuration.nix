{
  host,
  nix,
  ...
}: {
  imports = [
    ../../services
    ../../modules

    ./hardware-configuration.nix
  ];

  hardware = {
    enableRedistributableFirmware = true;
    cpu.intel.updateMicrocode = true;
  };

  time = {
    inherit (host) timeZone;
    # Without this option, the machine will have a UTC time
    hardwareClockInLocalTime = true;
  };

  i18n.defaultLocale = host.i18nLocale;
  location.provider = "geoclue2";

  system.activationScripts = {
    banner.text = ''
      echo '
          .              +   .                .   . .     .  .
                          .                    .       .     *
          .       *                        . . . .  .   .  + .
                    (You Are Here)            .   .  +  . . .
        .                 |             .  .   .    .    . .
                          |           .     .     . +.    +  .
                          ↓            .       .   . .
                . .       .         .    * . . .  .  +   .
                  +        ° <- (nyx)  *   .   .      +
                                    .       . +  .+. .
          .                      .     . + .  . .     .      .
                  .      .    .     . .   . . .        ! /
              *             .    . .  +    .  .       - O -  ←- (The sun)
                  .     .    .  +   . .  *  .        . / |
                      . + .  .  .  .. +  .
        .      .  .  .  *   .  *  . +..  .            *
        .      .   . .   .   .   . .  +   .    .            +
      '

      echo 'Remember to take all the blame!'
    '';
  };

  system = {
    inherit (nix) stateVersion;
    autoUpgrade = {
      enable = true;
      allowReboot = false;
      channel = "https://nixos.org/channels/${nix.channel}";
    };
  };
}

{
  host,
  nix,
  ...
}: {
  imports = [
    ../../modules
    ../../options

    ./hardware-configuration.nix
    ./disko-config.nix
    ./hostcmd.nix
  ];

  home-manager.useGlobalPkgs = true;

  hardware = {
    enableRedistributableFirmware = true;
    cpu.intel.updateMicrocode = true;
  };

  time.timeZone = host.timeZone;

  services.chrony.enable = true;

  i18n.defaultLocale = host.i18nLocale;
  # location.provider = "geoclue2";

  documentation.enable = false;

  system = {
    activationScripts = {
      banner.text = ''
        if [ ! -f /run/nixos-banner-shown ]; then
          echo '    .              +   .                .   . .     .  .'
          echo '                    .                    .       .     *'
          echo '    .       *                        . . . .  .   .  + .'
          echo '              (You Are Here)            .   .  +  . . .'
          echo '  .                 |             .  .   .    .    . .'
          echo '                    |           .     .     . +.    +  .'
          echo '                    ↓            .       .   . .'
          echo '         . .       . * <- (chimera).    * . . .  .  +   .'
          echo '            +        ° <- (nyx)  *   .   .      +'
          echo '<==[{         ?             .       . +  .+. .'
          echo '    .                      .     . + .  . .     .      .'
          echo '            .      .    .     . .   . . .        ! /'
          echo '        *             .    . .  +    .  .       - O -  ←- (rose)'
          echo '            .     .    .  +   . .  *  .        . / |'
          echo '                . + .  .  .  .. +  .'
          echo '  .  +   .  .  .  *   .  *  . +..  .            *'
          echo '  .      .   . .   .   .   . .  +   .    .            +'
          echo

          touch /run/nixos-banner-shown
        fi
      '';
    };

    nixos.tags = ["yay"];

    inherit (nix) stateVersion;
    autoUpgrade.enable = false;
  };
}

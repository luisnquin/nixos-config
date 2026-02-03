{
  host,
  pkgs,
  lib,
  nix,
  ...
}: {
  imports = [
    ../../services
    ../../modules

    ./hardware-configuration.nix
    ./disko-config.nix
  ];

  home-manager.useGlobalPkgs = true;

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

  documentation.enable = false;

  system = {
    activationScripts = {
      banner.text = ''
        echo '    .              +   .                .   . .     .  .'
        sleep 0.3
        echo '                    .                    .       .     *'
        echo '    .       *                        . . . .  .   .  + .'
        sleep 0.2
        echo '              (You Are Here)            .   .  +  . . .'
        echo '  .                 |             .  .   .    .    . .'
        echo '                    |           .     .     . +.    +  .'
        sleep 0.5
        echo '                    ↓            .       .   . .'
        echo '          . .       .         .    * . . .  .  +   .'
        echo '            +        ° <- (nyx)  *   .   .      +'
        echo '<==[{                      .       . +  .+. .'
        sleep 0.3
        echo '    .                      .     . + .  . .     .      .'
        echo '            .      .    .     . .   . . .        ! /'
        echo '        *             .    . .  +    .  .       - O -  ←- (jyx)'
        echo '            .     .    .  +   . .  *  .        . / |'
        echo '                . + .  .  .  .. +  .'
        echo '  .  +   .  .  .  *   .  *  . +..  .            *'
        echo '  .      .   . .   .   .   . .  +   .    .            +'
        echo

        ${lib.getExe pkgs.genact} -s=5 --exit-after-time=1s \
          -m bootlog \
          -m cryptomining \
          -m docker_image_rm \
          -m download \
          -m kernel_compile \
          -m memdump \
          -m simcity
      '';
    };

    nixos.tags = ["hyprland" "yay"];

    inherit (nix) stateVersion;
    autoUpgrade.enable = false;
  };
}

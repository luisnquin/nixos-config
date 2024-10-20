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
      echo '        *             .    . .  +    .  .       - O -  ←- (The sun)'
      echo '            .     .    .  +   . .  *  .        . / |'
      echo '                . + .  .  .  .. +  .'
      echo '  .  +   .  .  .  *   .  *  . +..  .            *'
      echo '  .      .   . .   .   .   . .  +   .    .            +'
      echo

      random_bool=$(( RANDOM % 2 ))
      i=0
      while [ $i -le 15 ]; do
        if [[ $random_bool -eq 0 ]]; then
          echo 'Remember to take all the blame!'
        else
          echo 'Take all the blame!'
        fi

        sleep 0.05
        i=$(( i + 1 ))
      done
      sleep 0.8
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

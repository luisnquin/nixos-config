{pkgs, ...}: {
  home.packages = with pkgs; [
    (ranger.overrideAttrs
      (r: {
        preConfigure =
          r.preConfigure
          + ''
            substituteInPlace ranger/ext/img_display.py \
              --replace "Popen(['ueberzug'" \
                        "Popen(['${ueberzugpp}/bin/ueberzugpp'"

            substituteInPlace ranger/config/rc.conf \
              --replace 'set preview_images_method w3m' \
                        'set preview_images_method ueberzug'
          '';
      }))
    gnome.nautilus
  ];
}

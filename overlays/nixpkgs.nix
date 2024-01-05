[
  (self: super: {
    ranger =
      super.ranger.overrideAttrs
      (r: {
        preConfigure =
          r.preConfigure
          + ''
            substituteInPlace ranger/ext/img_display.py \
              --replace "Popen(['ueberzug'" \
                        "Popen(['${self.ueberzugpp}/bin/ueberzugpp'"

            substituteInPlace ranger/config/rc.conf \
              --replace 'set preview_images_method w3m' \
                        'set preview_images_method ueberzug'
          '';
      });
  })
  # https://github.com/NotAShelf/nyx/blob/main/homes/notashelf/graphical/apps/discord/default.nix
  # TODO: check if we're using Wayland
  (self: super: {
    discord =
      (super.discord.override {
        nss = super.nss_latest;
        withOpenASAR = true;
      })
      .overrideAttrs
      (old: {
        libPath = old.libPath + ":${super.libglvnd}/lib";
        nativeBuildInputs = old.nativeBuildInputs ++ [super.makeWrapper];
        postFixup = ''
          wrapProgram $out/opt/Discord/Discord --add-flags "\''${NIXOS_OZONE_WL:+\''${WAYLAND_DISPLAY:+--ozone-platform=wayland}}"
        '';
      });
  })
]

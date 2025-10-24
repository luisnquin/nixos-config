{
  inputs,
  system,
}: [
  (
    _self: pkgs: {
      express-lrs-configurator = let
        # name = "ExpressLRS-Configurator";
        pname = "expresslrs-configurator";
        version = "1.7.10";

        dist = pkgs.fetchzip {
          name = "${pname}-dist";
          url = "https://github.com/ExpressLRS/ExpressLRS-Configurator/releases/download/v${version}/${pname}-${version}.zip";
          stripRoot = false;
          hash = "sha256-RHQTCX+7OIzw/LGKvSgoLfffFN5V/kYykUzUtwKKHb8=";
        };

        desktopItem = pkgs.makeDesktopItem {
          name = pname;
          exec = pname;
          icon = pname;
          comment = "Cross platform configuration & build tool for the ExpressLRS radio link";
          desktopName = "ExpressLRS Configurator";
          genericName = "radio link configuration & build tool";
        };
      in
        pkgs.buildFHSEnv {
          inherit pname version;
          # copied from chromium deps
          targetPkgs = pkgs: (with pkgs; [
            glib
            fontconfig
            freetype
            pango
            cairo
            xorg.libX11
            xorg.libXi
            atk
            nss
            nspr
            xorg.libXcursor
            xorg.libXext
            xorg.libXfixes
            xorg.libXrender
            xorg.libXScrnSaver
            xorg.libXcomposite
            xorg.libxcb
            alsa-lib
            xorg.libXdamage
            xorg.libXtst
            xorg.libXrandr
            xorg.libxshmfence
            expat
            cups
            dbus
            gdk-pixbuf
            gcc-unwrapped.lib
            systemd
            libexif
            pciutils
            liberation_ttf
            curl
            util-linux
            wget
            flac
            harfbuzz
            icu
            libpng
            libopus
            snappy
            speechd
            bzip2
            libcap
            at-spi2-atk
            at-spi2-core
            libkrb5
            libdrm
            libgbm
            libglvnd
            mesa
            coreutils
            libxkbcommon
            pipewire
            wayland
            libva
            gtk3
            gtk4
          ]);
          extraInstallCommands = ''
            mkdir -p $out/share
            ${pkgs.imagemagick}/bin/convert ${dist}/resources/assets/icon.png -resize 128x128 icon-128.png
            install -m 444 -D icon-128.png $out/share/icons/hicolor/128x128/apps/${pname}.png
            cp -r ${desktopItem}/share/applications $out/share/
          '';

          runScript = "${dist}/expresslrs-configurator";
        };
    }
  )
  (
    _self: super: {
      panicparse = super.panicparse.overrideAttrs (_old: {
        postInstall = ''
          mv $out/bin/panicparse $out/bin/pp
        '';
      });
    }
  )
  (_self: _super: {
    inherit (inputs.hyprland.packages.${system}) xdg-desktop-portal-hyprland;

    hyprland = inputs.hyprland.packages.${system}.hyprland.overrideAttrs (_oldAttrs: {
      # disko does not work with the src they've set
      src = _self.fetchgit {
        url = "https://github.com/hyprwm/Hyprland";
        rev = "aa5a239ac92a6bd6947cce2ca3911606df392cb6";
        sha256 = "sha256-KDy8Vtlwe+7Z053HtD4fCRqlHBt0Kils0Zea4D77R7o=";
      };
    });
  })
]

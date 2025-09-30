{
  inputs,
  system,
}: [
  (
    _self: super: {
      logkeys = super.logkeys.overrideAttrs (prev: {
        postPatch =
          prev.postPatch
          + ''
            mkdir -p $out/etc/logkeys/keymaps
            cp $src/keymaps/es_ES.map $out/etc/logkeys/keymaps

            substituteInPlace scripts/logkeys-start.sh \
              --replace 'logkeys --start' 'logkeys --keymap $out/etc/logkeys/keymaps/es_ES.map --start'
          '';
      });
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
      src = _self.fetchgit {
        url = "https://github.com/hyprwm/Hyprland";
        rev = "38c1e72c9d81fcdad8f173e06102a5da18836230";
        sha256 = "sha256-SAJKAYq1QeDCx19+JVwkvyfXLpmXJrOyUCRH+Dy7T/c=";
      };
    });
  })
]

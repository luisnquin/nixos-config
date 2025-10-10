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
      # disko does not work with the src they've set
      src = _self.fetchgit {
        url = "https://github.com/hyprwm/Hyprland";
        rev = "b965fb2a40b132209b58f511e2604a2939461818";
        sha256 = "sha256-euUD98tQXJ3nxlBK6Sb/EnjWoJPmXyzbKJRbgyVgErk=";
      };
    });
  })
]

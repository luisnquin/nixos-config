{
  inputs,
  system,
}: [
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
    inherit (inputs.hyprdysmorphic.packages.${system}) xdg-desktop-portal-hyprland hyprlauncher hyprland;
  })
]

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

  (_self: super: {
    antigravity = super.antigravity.overrideAttrs (_oldAttrs: rec {
      version = "1.19.6";
      src = super.fetchurl {
        url = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/${version}-6514342219874304/linux-x64/Antigravity.tar.gz";
        sha256 = "sha256-gFIsnWC8wEuxPUD6E2YB0YTcg/NruQZespzEVttMKeE=";
      };
    });
  })
]

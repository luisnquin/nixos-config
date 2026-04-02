{...}: [
  (
    _self: super: {
      panicparse = super.panicparse.overrideAttrs (_old: {
        postInstall = ''
          mv $out/bin/panicparse $out/bin/pp
        '';
      });
    }
  )
  (_self: super: {
    antigravity = super.antigravity.overrideAttrs (_oldAttrs: rec {
      version = "1.21.9";
      src = super.fetchurl {
        url = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/${version}-4905428782546944/linux-x64/Antigravity.tar.gz";
        sha256 = "sha256-hepNVfUtMvu/nZL93HR/EOjQTBvQCgdyG1cfp/LvUiY=";
      };
    });
  })
]

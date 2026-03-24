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
      version = "1.20.6";
      src = super.fetchurl {
        url = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/${version}-5891862175809536/linux-x64/Antigravity.tar.gz";
        sha256 = "sha256-rTgr8yGmIW0H+Vrx9hPgP1oH/fb8ZjK3ac6D2Br91Wc=";
      };
    });
  })
]

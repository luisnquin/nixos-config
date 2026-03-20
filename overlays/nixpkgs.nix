{
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
  (_self: super: {
    antigravity = super.antigravity.overrideAttrs (_oldAttrs: rec {
      version = "1.20.5";
      src = super.fetchurl {
        url = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/${version}-5474622945755136/linux-x64/Antigravity.tar.gz";
        sha256 = "sha256-W4dmT0VNpIe43uh6r14zYdm6eblHdwt5i9D6h0qYJ+U=";
      };
    });
  })
]

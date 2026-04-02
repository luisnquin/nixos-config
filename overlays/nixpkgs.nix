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
  (_self: super: {
    rtk = super.rtk.overrideAttrs (_oldAttrs: {
      postInstall = ''
        mkdir -p $out/share/rtk/hooks
        cp -r hooks/* $out/share/rtk/hooks/
      '';
    });
  })
  (_self: super: {
    claude-code = super.buildNpmPackage (finalAttrs: {
      pname = "claude-code";
      version = "2.1.90";

      src = super.fetchzip {
        url = "https://registry.npmjs.org/@anthropic-ai/claude-code/-/claude-code-${finalAttrs.version}.tgz";
        hash = "sha256-4/hqWrY2fncQ8p0TxwBAI+mNH98ZDhjvFqB9us7GJK0=";
      };

      npmDepsHash = "sha256-8+i2Ay9bGSH1QxAKy5kQ0d57zKlhmTixz3he2ce3KXY=";

      postPatch = ''
        cp ${./claude-code-package-lock.json} package-lock.json
      '';

      dontNpmBuild = true;
      env.AUTHORIZED = "1";
    });
  })
]

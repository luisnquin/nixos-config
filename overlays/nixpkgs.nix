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
    claude-code = super.buildNpmPackage (finalAttrs: {
      pname = "claude-code";
      version = "2.1.89";

      src = super.fetchzip {
        url = "https://registry.npmjs.org/@anthropic-ai/claude-code/-/claude-code-${finalAttrs.version}.tgz";
        hash = "sha256-FoTm6KDr+8Dzhk4ibZUlU1QLPFdPm/OriUUWqAaFswg=";
      };

      npmDepsHash = "sha256-NI4F5bq0lEuMjLUdkGrml2aOzGbGkdyUckgfeVFEe8o=";

      postPatch = ''
        cp ${./claude-code-package-lock.json} package-lock.json
      '';

      dontNpmBuild = true;
      env.AUTHORIZED = "1";
    });
  })
]

{...}: [
  (
    _self: super: {
      openldap = super.openldap.overrideAttrs (_oldAttrs: {
        doCheck = false;
      });

      panicparse = super.panicparse.overrideAttrs (_old: {
        postInstall = ''
          mv $out/bin/panicparse $out/bin/pp
        '';
      });
    }
  )
  (_self: super: {
    antigravity = super.antigravity.overrideAttrs (oldAttrs: rec {
      version = "2.0.4";

      src = super.fetchurl {
        url = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/${version}-6381998290370560/linux-x64/Antigravity%20IDE.tar.gz";
        sha256 = "sha256-ZjN9RfJHLOXonzlOd67HSQmqG+C7M8n3MpmpX0WOZ3A=";
      };

      sourceRoot = "Antigravity IDE";

      installPhase =
        super.lib.replaceStrings
        [
          ''"$out/lib/antigravity/bin/antigravity"''
          ''"$out/bin/antigravity"''
        ]
        [
          ''"$out/lib/antigravity/bin/antigravity-ide"''
          ''"$out/bin/antigravity-ide"''
        ]
        oldAttrs.installPhase;

      postFixup =
        super.lib.replaceStrings
        ["$out/lib/antigravity/antigravity"]
        ["$out/lib/antigravity/antigravity-ide"]
        oldAttrs.postFixup;
    });
  })
  (final: _prev: {
    rtk = final.llm-agents.rtk.overrideAttrs (_oldAttrs: {
      postInstall = ''
        mkdir -p $out/share/rtk/hooks
        cp -r hooks/* $out/share/rtk/hooks/
      '';
    });
  })
]

{...}: [
  (
    _self: super: {
      openldap = super.openldap.overrideAttrs (_oldAttrs: {
        doCheck = false;
      });

      waybar = super.waybar.overrideAttrs (_oldAttrs: let
        libcavaVersion = "0.10.7";
        libcavaSrc = super.fetchFromGitHub {
          owner = "LukashonakV";
          repo = "cava";
          rev = libcavaVersion;
          hash = "sha256-zkyj1vBzHtoypX4Bxdh1Vmwh967DKKxN751v79hzmgQ=";
        };
      in {
        src = super.fetchFromGitHub {
          owner = "Alexays";
          repo = "Waybar";
          rev = "05945748dccce28bf96d26d8f64a9e69a8dd49ba";
          hash = "sha256-51R3mIt8cLNvh/X5qe9vOqeJCj0U9KRyemVE5y+OhiU=";
        };

        postUnpack = ''
          pushd "$sourceRoot"
          cp -R --no-preserve=mode,ownership ${libcavaSrc} subprojects/cava-${libcavaVersion}
          patchShebangs .
          popd
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

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
    antigravity = super.antigravity.overrideAttrs (_oldAttrs: rec {
      version = "1.23.2";

      src = super.fetchurl {
        url = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/${version}-4781536860569600/linux-x64/Antigravity.tar.gz";
        sha256 = "sha256-UjKkBI/0+hVoXZqYG6T7pXPil/PvybdvY455S693VyU=";
      };
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
  (_self: super: let
    libcavaSrc = super.fetchFromGitHub {
      owner = "LukashonakV";
      repo = "cava";
      rev = "0.10.7";
      hash = "sha256-zkyj1vBzHtoypX4Bxdh1Vmwh967DKKxN751v79hzmgQ=";
    };
  in {
    waybar = super.waybar.overrideAttrs (_oldAttrs: {
      version = "0.15.0";

      src = super.fetchFromGitHub {
        owner = "Alexays";
        repo = "Waybar";
        rev = "05945748dccce28bf96d26d8f64a9e69a8dd49ba";
        hash = "sha256-51R3mIt8cLNvh/X5qe9vOqeJCj0U9KRyemVE5y+OhiU=";
      };

      postUnpack = ''
        pushd "$sourceRoot"
        cp -R --no-preserve=mode,ownership ${libcavaSrc} subprojects/cava-0.10.7
        patchShebangs .
        popd
      '';
    });
  })
]

{
  inputs,
  system,
  host,
  ...
}: [
  (_final: _prev: {
    home-manager = inputs.home-manager.packages.${system}.home-manager.overrideAttrs (old: {
      buildCommand =
        old.buildCommand
        + ''
          substituteInPlace $out/bin/home-manager \
            --replace-fail "    presentNews" "    :"
        '';
    });
  })
  (
    _self: super: {
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

        doCheck = false;
        mesonFlags =
          map
          (flag:
            if flag == "-Dtests=enabled"
            then "-Dtests=disabled"
            else flag)
          (_oldAttrs.mesonFlags or []);
      });
    }
  )
  (_self: super: {
    antigravity = super.antigravity.overrideAttrs (oldAttrs: rec {
      version = "2.1.1";

      src = super.fetchurl {
        url = "https://edgedl.me.gvt1.com/edgedl/release2/j0qc3/antigravity/stable/${version}-6123990880747520/linux-x64/Antigravity%20IDE.tar.gz";
        sha256 = "sha256-Wyzr99M6aNAD/Y8fqYjRYAkFrOIlBKCF5ThCFCkIeL0=";
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
  (final: prev: {
    # aa-remove-unknown sources rc.apparmor.functions from apparmor-parser, which
    # no longer ships it after the package split (only apparmor-init does), so
    # apparmor.service reload fails and switch-to-configuration exits 4.
    # Fixed upstream in nixpkgs 6bea8bbfd; drop once nixos-unstable includes it.
    apparmor-utils = prev.apparmor-utils.overrideAttrs (old: {
      postInstall =
        (old.postInstall or "")
        + ''
          substituteInPlace $out/bin/.aa-remove-unknown-wrapped \
            --replace-fail "${final.apparmor-parser}/lib/apparmor/rc.apparmor.functions" \
                           "${final.apparmor-init}/lib/apparmor/rc.apparmor.functions"
        '';
    });
  })
  (_final: prev: {
    # Temporary pin past upstream #510; drop once nixpkgs ships > 0.8.1.
    codebase-memory-mcp = prev.codebase-memory-mcp.overrideAttrs (_old: rec {
      version = "0.8.1-unstable-2026-06-29";
      src = prev.fetchFromGitHub {
        owner = "DeusData";
        repo = "codebase-memory-mcp";
        rev = "7824e505c192023a21b3e90bcb98ca6210629b64";
        hash = "sha256-wAxnaSnZylJTbvV0rn+4mzDlKtJDaIipYoOdjdqTsLE=";
      };
      npmDeps = prev.fetchNpmDeps {
        inherit src;
        sourceRoot = "${src.name}/graph-ui";
        hash = "sha256-P1JVo+GFr+Gsq88dnn9OsedZqrTaj3DDGbej+nHbp4U=";
      };
      buildPhase = ''
        runHook preBuild
        make -j$NIX_BUILD_CORES -f Makefile.cbm cbm CFLAGS_EXTRA='-DCBM_VERSION=\"${version}\"'
        runHook postBuild
      '';
    });
  })
  (_final: prev: {
    browsers = prev.browsers.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/browsers/zen-linux-app.patch
          ./patches/browsers/larger-picker-window.patch
          ./patches/browsers/fix-url-argument-injection.patch
        ];
    });
  })
  (_final: prev: {
    # Must stay after hyprdysmorphic, which replaces `hyprland` outright.
    # The grep turns a moved banner into a build failure instead of a no-op.
    hyprland = prev.hyprland.overrideAttrs (old: {
      postPatch =
        (old.postPatch or "")
        + ''
          grep -q 'std::println(R"#($' src/main.cpp
          sed -i \
            -e '/std::println(R"#($/,/^)#");$/{/std::println(R"#($/!{/^)#");$/!d}}' \
            -e '/std::println(R"#($/r ${prev.writeText "hyprland-banner" host.banner}' \
            src/main.cpp
        '';
    });
  })
]

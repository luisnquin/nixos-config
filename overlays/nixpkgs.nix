{host, ...}: [
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
    # 3.7b's window_tree_build reads `s = l[n - 1]` once outside the loop, so the
    # session-group check runs against the last session for every iteration and
    # `continue` skips all of them: choose-tree/choose-window/choose-session draw
    # nothing whenever a grouped session exists (ssh-gateway makes one). Fixed
    # upstream after 3.7b; drop once nixpkgs ships 3.8.
    tmux = prev.tmux.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/tmux/window-tree-per-session-group-check.patch
        ];
    });
  })
  (_final: prev: {
    # Must stay after hyprdysmorphic, which replaces `hyprland` outright.
    # The grep turns a moved banner into a build failure instead of a no-op.
    hyprland = prev.hyprland.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/hyprland/scrolling-fs-restore-offset.patch
        ];
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
  (final: _prev: {
    spiceedit = final.buildGoModule {
      pname = "spiceedit";
      version = "0.0.43";

      src = final.fetchFromGitHub {
        owner = "cloudmanic";
        repo = "spice-edit";
        rev = "v0.0.43";
        hash = "sha256-SJ/q7mg6toKbYJjSl1uFH79LR6auxUxguGuXW3kAiDs=";
      };

      vendorHash = "sha256-rjmk+9Yz3riXfvCERs6noGuVOFyEt8SoHbxjAt7D2IY=";

      env.CGO_ENABLED = 0;
      ldflags = ["-s" "-w"];

      postInstall = ''
        mv $out/bin/spice-edit $out/bin/spiceedit
      '';

      meta = {
        description = "Opinionated mouse-first terminal code editor";
        homepage = "https://github.com/cloudmanic/spice-edit";
        license = final.lib.licenses.mit;
        mainProgram = "spiceedit";
      };
    };
  })
  (final: _prev: {
    herdr-autoname = final.rustPlatform.buildRustPackage {
      pname = "herdr-autoname";
      version = "0.1.0";

      src = final.lib.cleanSource ./herdr-autoname;

      cargoLock.lockFile = ./herdr-autoname/Cargo.lock;

      postInstall = ''
        cp herdr-plugin.toml $out/
        install -Dm644 shell/hook.zsh $out/shell/hook.zsh
      '';

      meta.mainProgram = "herdr-autoname";
    };
  })
  (final: _prev: {
    # Mic92's OSC 52 fork of rmarganti/herdr-pluck: clipboard survives SSH panes.
    herdr-pluck = final.rustPlatform.buildRustPackage {
      pname = "herdr-pluck";
      version = "0.1.0-unstable-2026-07-23";

      src = final.fetchFromGitHub {
        owner = "Mic92";
        repo = "herdr-pluck";
        rev = "6f94c5b2e41e3f51a868847d7a62f140c4ff496c";
        hash = "sha256-7MyNBAHUbimRd68Oj8d9Y2l4knmHMqHNNdUtBJOkwJM=";
      };

      cargoHash = "sha256-h3yU5gPuJSdv4fW8kbfCxdAR0Nnnr5/dYTNaMhNNFIE=";

      postInstall = ''
        cp herdr-plugin.toml $out/
      '';

      meta.mainProgram = "herdr-pluck";
    };
  })
  (final: _prev: {
    herdr-sesh = final.buildGoModule rec {
      pname = "herdr-sesh";
      version = "0.5.0";

      src = final.fetchFromGitHub {
        owner = "fullerzz";
        repo = "herdr-plugin-sesh";
        rev = "v${version}";
        hash = "sha256-IGLMExUtNI8ybwY0tOVzhxZSFl5SJgu98DW+kvcBTyY=";
      };

      vendorHash = "sha256-TnfuQetN3KaRsB5r1bTCcQwOw6kqYVjzKb2aWkz6C0A=";

      subPackages = ["cmd/herdr-sesh"];

      ldflags = ["-X=github.com/fullerzz/herdr-plugin-sesh/internal/app.Version=${version}"];

      postInstall = ''
        cp herdr-plugin.toml $out/
      '';

      meta.mainProgram = "herdr-sesh";
    };
  })
]

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
  (final: _prev: {
    rtk = final.llm-agents.rtk.overrideAttrs (_oldAttrs: {
      postInstall = ''
        mkdir -p $out/share/rtk/hooks
        cp -r hooks/* $out/share/rtk/hooks/
      '';
    });
  })
  (_final: prev: rec {
    codex = prev.llm-agents.codex.overrideAttrs (old: let
      devServerInstruction = "When building a site or app that needs a dev server to run properly, you start the local dev server after implementation and give the user the URL so they can try it. If there's already a server on that port, you use another one. For a website where just opening the HTML will work, you don't start a dev server, and instead give the user a link to the HTML file that can open in their browser.\\n\\n";
    in {
      patches =
        (old.patches or [])
        ++ [
          ./patches/codex/recursive-project-trust.patch
          ./patches/codex/git-remote-show-no-fetch.patch
          ./patches/codex/curated-plugins-disable-sync.patch
          ./patches/codex/presentation-card.patch
          ./patches/codex/terminal-terminfo-dirs.patch
          ./patches/codex/custom-input-bar.patch
          ./patches/codex/disable-cloud-tasks.patch
          ./patches/codex/default-bypass-hook-trust.patch
          ./patches/codex/disable-update-advice.patch
          ./patches/codex/session-only-directory-trust.patch
          ./patches/codex/disable-config-toml-writes.patch
          ./patches/codex/default-yolo.patch
          ./patches/codex/status-line-short-cwd.patch
          ./patches/codex/status-line-effective-reasoning.patch
          ./patches/codex/unload-unsubscribed-threads.patch
        ];

      postPatch =
        (old.postPatch or "")
        + ''
          substituteInPlace models-manager/models.json \
            --replace-fail ${prev.lib.escapeShellArg devServerInstruction} ""
        '';
    });

    llm-agents = prev.llm-agents // {inherit codex;};
  })
  (_final: prev: {
    mako = prev.mako.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/mako/notification-metadata.patch
          ./patches/mako/restore-history-by-id.patch
          ./patches/mako/history-actions-and-removal.patch
        ];
    });
  })
  (_final: prev: {
    lazygit = prev.lazygit.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/lazygit/hide-personal-authors.patch
          ./patches/lazygit/shift-delete-in-text-inputs.patch
          ./patches/lazygit/submodules-dashboard.patch
        ];
    });
  })
  (_final: prev: {
    # Temporary pin past upstream #510; drop once nixpkgs ships 0.9.1 or newer.
    # print_help and handle_subcommand both moved, so nixpkgs' own
    # remove-install-update.diff no longer applies and is replaced by a copy
    # refreshed against this rev — the self-install/update subcommands still
    # have to go, they would write into a store-managed install.
    codebase-memory-mcp = prev.codebase-memory-mcp.overrideAttrs (_old: rec {
      version = "0.9.1-rc.1-unstable-2026-07-31";
      src = prev.fetchFromGitHub {
        owner = "DeusData";
        repo = "codebase-memory-mcp";
        rev = "d6be58ef9d43c574a2d1b0827ecc1e3c4846f0fe";
        hash = "sha256-4Z3DjDXOM3XMMdzz1aZQIAO2qjhlNXiCpTw1faltH30=";
      };
      npmDeps = prev.fetchNpmDeps {
        inherit src;
        sourceRoot = "${src.name}/graph-ui";
        hash = "sha256-6NUv6JCUAPHmq7RbgkgaQVrKgeL09QfUksz4uvh5UAA=";
      };
      patches = [
        ./patches/codebase-memory-mcp/remove-install-update.patch
      ];
      # embed-frontend.sh moved to `#!/usr/bin/env bash`, so nixpkgs' hardcoded
      # /bin/bash --replace-fail has nothing left to hit; patchShebangs resolves
      # it instead. The npm ci strip stays, npmDeps already vendored the tree.
      postPatch = ''
        substituteInPlace Makefile.cbm \
          --replace-fail "npm ci &&" ""

        patchShebangs scripts/embed-frontend.sh
      '';
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
  (_final: prev: {
    # The sidebar collapse toggle is a one-cell icon in the bottom-right corner:
    # unhittable with a finger over ssh from a phone. The icon stays where it is,
    # only its press area grows — left and up, never onto the divider column,
    # which upstream keeps draggable down to its last row.
    #
    # The second patch teaches herdr about freebuff, which upstream does not
    # know at all. Two halves: the resume planner checks every session source
    # against a hardcoded allowlist, so freebuff chats were refused before they
    # could be persisted; and `Agent` is a closed enum, so a freebuff pane
    # rendered nameless in the agents sidebar with no state of its own. It pairs
    # with the freebuff-side patch that reports the chat id, since freebuff has
    # no hook or plugin surface for herdr to install into.
    herdr = prev.herdr.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/herdr/larger-sidebar-toggle-hit-area.patch
          ./patches/herdr/freebuff-agent-session.patch
        ];
    });
  })
  # Everything ./pkgs defines — first-party and packaged upstreams alike —
  # lands on `pkgs.<name>`, so no module has to be wired its own way.
  (final: _prev: import ../pkgs final)
  (_final: prev: {
    # Must stay after the ./pkgs import, which is what defines `linear-tui`.
    # Upstream drops image links on the floor; the first patch resolves them
    # against the Kitty graphics protocol, which ghostty speaks. The second
    # trades the issue table's empty planning columns for the title and an
    # attachment marker. The third collapses the state and priority columns
    # into single severity glyphs.
    linear-tui = prev.linear-tui.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/linear-tui/terminal-image-rendering.patch
          ./patches/linear-tui/issue-table-layout.patch
          ./patches/linear-tui/issue-table-status-icons.patch
        ];
    });
  })
]

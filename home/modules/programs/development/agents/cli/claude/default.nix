{
  mkAgentKit,
  config,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};
  sources = builtins.mapAttrs (_: pkgs.fetchFromGitHub) (lib.importJSON ./plugins-sources.json);
in {
  imports = [
    ./hooks.nix
  ];

  # not `programs.claude-code.plugins`: its per-entry links put mod files
  # outside the plugin directory, which claude refuses. Copied, not linked:
  # claude reloads a mod whenever a watched file's ctime moves, and with
  # auto-optimise-store every dirty-tree flake copy hardlinks the same inodes
  home.activation.claudeMods = let
    skills = "${config.programs.claude-code.configDir}/skills";
    stamps = "${config.xdg.stateHome}/claude-mods";
  in
    lib.hm.dag.entryAfter ["linkGeneration"] ''
      claudeMod() {
        local dst="${skills}/$1" stamp="${stamps}/$1"
        [ "$(cat "$stamp" 2>/dev/null)" = "$2" ] && [ -d "$dst" ] && [ ! -L "$dst" ] && return
        run rm -rf "$dst.new"
        run cp -rL --no-preserve=mode,ownership "$2" "$dst.new"
        run rm -rf "$dst"
        run mv "$dst.new" "$dst"
        [[ -v DRY_RUN ]] || printf '%s\n' "$2" > "$stamp"
      }
      run mkdir -p "${skills}" "${stamps}"
      claudeMod phone ${pkgs.phone.claudePlugin}
      claudeMod chat-clean ${sources.chat-clean}
    '';

  xdg.configFile."ccstatusline/settings.json" = let
    settingsJson = builtins.fromJSON (builtins.readFile ./ccstatusline-settings.json);
  in {
    text = builtins.toJSON (settingsJson
      // {
        installation = {
          method = "pinned";
          installedVersion = pkgs.llm-agents.ccstatusline.version;
        };
      });
  };

  programs.claude-code = {
    enable = true;
    package = pkgs.llm-agents.claude-code;
    enableMcpIntegration = true;
    mcpServers = kit.mkMcpServers {};

    hooks = {
      "rtk-rewrite.sh" = let
        upstream = builtins.readFile "${pkgs.rtk}/share/rtk/hooks/claude/rtk-rewrite.sh";
        autoAllow = lib.concatMapStrings (line: "        ${line}\n") [
          ''"permissionDecision": "allow",''
          ''"permissionDecisionReason": "RTK auto-rewrite",''
        ];
        rewritten = "EXIT_CODE=$?\n";
        # a worktree-isolated agent's git calls are refused once rtk wraps them
        isolatedGit = ''
          case "$(jq -r '.cwd // empty' <<<"$INPUT")" in
            */.claude/worktrees/*) case "$REWRITTEN" in *"rtk git"*) exit 0 ;; esac ;;
          esac
        '';
      in
        assert lib.assertMsg (lib.hasInfix autoAllow upstream) "rtk-rewrite.sh changed; re-check its auto-allow branch";
        assert lib.assertMsg (lib.hasInfix rewritten upstream) "rtk-rewrite.sh changed; re-check where EXIT_CODE is read";
          builtins.replaceStrings [autoAllow rewritten] ["" (rewritten + isolatedGit)] upstream;
    };

    marketplaces = {
      inherit (sources) claude-plugins-official claude-image-view oikon48;
    };

    context = ''
      ${kit.memories}

      ${kit.claudeMemories}
    '';

    # https://code.claude.com/docs/en/settings#available-settings
    settings = {
      enabledPlugins = {
        "image-view@claude-image-view" = true;
        "prompt-rail@oikon48" = true;
        "rust-analyzer-lsp@claude-plugins-official" = true;
        "swift-lsp@claude-plugins-official" = true;
      };

      model = "opus";
      effortLevel = "high";
      language = "english";
      cleanupPeriodDays = 90;
      tui = "fullscreen";
      showThinkingSummaries = true;
      autoCompactWindow = 200000;

      env = {
        "DISABLE_TELEMETRY" = "1";
        "DISABLE_ERROR_REPORTING" = "1";
        "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS" = "1";
        "DISABLE_AUTOUPDATER" = "1";
        "PINENTRY_USER_DATA" = "gui";
      };

      companyAnnouncements = [
        "Reminder: you're in solo mode"
      ];

      attribution = false;
      skipDangerousModePermissionPrompt = true;

      statusLine = {
        "type" = "command";
        "command" = lib.getExe pkgs.llm-agents.ccstatusline;
        "padding" = 0;
        "refreshInterval" = 5;
      };

      permissions = kit.mkAgentPermissions "claude" {};
    };
  };
}

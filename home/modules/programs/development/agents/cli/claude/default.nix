{
  mkAgentKit,
  config,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};
in {
  imports = [
    ./hooks.nix
  ];

  # not `programs.claude-code.plugins`: its per-entry links put mod files
  # outside the plugin directory, which claude refuses
  home.file."${config.programs.claude-code.configDir}/skills/phone".source = pkgs.phone.claudePlugin;

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
      claude-plugins-official = pkgs.fetchFromGitHub {
        owner = "anthropics";
        repo = "claude-plugins-official";
        rev = "b091cb4179d3b62a6e2a39910461c7ec7165b1ef";
        sha256 = "sha256-uKDVcw6C1uzpiIY+hjgHxr4AU9wM1KF7t3v6zd9XBHk=";
      };
      claude-image-view = pkgs.fetchFromGitHub {
        owner = "jarrodwatts";
        repo = "claude-image-view";
        rev = "b3c412bb6d167cafade79148e95f9114ee1aad7c";
        sha256 = "sha256-WCJJrInTO6iOrcrFIm2adgg+3J0L8J+KNANanYSB3Rc=";
      };
    };

    context = ''
      ${kit.memories}

      ${kit.claudeMemories}
    '';

    # https://code.claude.com/docs/en/settings#available-settings
    settings = {
      enabledPlugins = {
        "image-view@claude-image-view" = true;
        "rust-analyzer-lsp@claude-plugins-official" = true;
        "swift-lsp@claude-plugins-official" = true;
      };

      model = "opus";
      effortLevel = "high";
      language = "english";
      cleanupPeriodDays = 20;
      tui = "fullscreen";

      env = {
        "CLAUDE_CODE_AUTO_COMPACT_WINDOW" = "200000";
        "CLAUDE_CODE_ENABLE_TELEMETRY" = "0";
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

{
  config,
  agent,
  pkgs,
  ...
}: let
  inherit (agent) mkAudioHook mkNotificationHook;
  inherit (agent.assets) sounds images;
in {
  programs.claude-code = {
    enable = true;
    enableMcpIntegration = true;
    mcpServers = builtins.removeAttrs config.programs.mcp.servers [
      "supabase"
    ];

    hooks = {
      "rtk-rewrite.sh" = builtins.readFile "${pkgs.rtk}/share/rtk/hooks/rtk-rewrite.sh";
    };

    marketplaces = {
      claude-plugins-official = pkgs.fetchFromGitHub {
        owner = "anthropics";
        repo = "claude-plugins-official";
        rev = "b091cb4179d3b62a6e2a39910461c7ec7165b1ef";
        sha256 = "sha256-uKDVcw6C1uzpiIY+hjgHxr4AU9wM1KF7t3v6zd9XBHk=";
      };
    };

    memory.text = agent.memories;

    # https://code.claude.com/docs/en/settings#available-settings
    settings = {
      enabledPlugins = {
        "rust-analyzer-lsp@claude-plugins-official" = true;
      };

      model = "claude-haiku-4-5-20251001";
      effortLevel = "medium";
      outputStyle = "Explanatory";
      language = "english";
      cleanupPeriodDays = 20;

      env = {
        "CLAUDE_CODE_AUTO_COMPACT_WINDOW" = "200000";
        "CLAUDE_CODE_ENABLE_TELEMETRY" = "0";
        "DISABLE_AUTOUPDATER" = "1";
      };

      hooks = {
        Notification = [
          (mkNotificationHook images.claude "Claude Code" "Awaiting your input")
          (mkAudioHook [sounds.buzact])
        ];
        SessionStart = [
          (mkAudioHook [sounds.ifarm])
        ];
        Elicitation = [
          (mkAudioHook [sounds.ifrtho])
        ];
        ElicitationResult = [
          (mkAudioHook [sounds.ifrtfy])
        ];
        PostToolUseFailure = [
          (mkAudioHook [sounds.ifdngr sounds.ifvfrs])
        ];
        UserPromptSubmit = [
          (mkAudioHook [sounds.ifrsig])
        ];
        TaskCompleted = [
          (mkAudioHook [sounds.ifgood sounds.ifrtho])
        ];
        StopFailure = [
          (mkAudioHook [sounds.ifdngr sounds.ifrsis])
        ];
        PermissionDenied = [
          (mkAudioHook [sounds.ifdngr sounds.permission-denied])
        ];
        PermissionRequest = [
          (mkNotificationHook images.claude "Claude Code" "Permission required")
          (mkAudioHook [sounds.ifdngr sounds.permission-required])
        ];
        PreToolUse = [
          {
            matcher = "Bash";
            hooks = [
              {
                type = "command";
                command = config.programs.claude-code.hooks."rtk-rewrite.sh";
              }
            ];
          }
        ];
        SessionEnd = [
          (mkAudioHook [sounds.ifdarm])
        ];
      };

      companyAnnouncements = [
        "Reminder: you're in solo mode"
      ];

      includeCoAuthoredBy = false;

      permissionProfile = "standard";
      permissions = {
        defaultMode = pkgs.lib.mkForce "acceptEdits";
        disableBypassPermissionsMode = "disable";
      };
    };
  };
}

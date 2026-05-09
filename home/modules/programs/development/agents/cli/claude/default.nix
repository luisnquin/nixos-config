{
  config,
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
in {
  programs.claude-code = {
    enable = true;
    package = pkgs.llm-agents.claude-code;
    enableMcpIntegration = true;
    mcpServers = kit.mkMcpServers {};

    hooks = {
      "rtk-rewrite.sh" = builtins.readFile "${pkgs.rtk}/share/rtk/hooks/claude/rtk-rewrite.sh";
    };

    marketplaces = {
      claude-plugins-official = pkgs.fetchFromGitHub {
        owner = "anthropics";
        repo = "claude-plugins-official";
        rev = "b091cb4179d3b62a6e2a39910461c7ec7165b1ef";
        sha256 = "sha256-uKDVcw6C1uzpiIY+hjgHxr4AU9wM1KF7t3v6zd9XBHk=";
      };
    };

    context = kit.memories;

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
        "PINENTRY_USER_DATA" = "gui";
      };

      hooks = {
        Notification = [
          (kit.mkNotificationEntry {
            image = kit.images.claude;
            title = "Claude Code";
            message = "Awaiting your input";
            ntfy = {
              delay = "10s";
              sequenceId = "claude-awaiting-input";
            };
            extraHooks = [
              (kit.mkAudioHook [kit.sounds.buzact])
            ];
          })
        ];
        SessionStart = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifarm];
          })
        ];
        Elicitation = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifrtho];
          })
        ];
        ElicitationResult = [
          (kit.mkHookEntry {
            hooks = [
              {
                type = "command";
                command = ''
                  ${kit.mkCancelNotificationCmd {
                    sequenceId = "claude-awaiting-input";
                  }}

                  ${kit.mkAudioCmd [kit.sounds.ifrtfy]}
                '';
              }
            ];
          })
        ];
        PostToolUseFailure = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifvfrs];
          })
        ];
        UserPromptSubmit = [
          (kit.mkHookEntry {
            hooks = [
              {
                type = "command";
                command = ''
                  ${kit.mkCancelNotificationCmd {
                    sequenceId = "claude-awaiting-input";
                  }}

                  ${kit.mkAudioCmd [kit.sounds.ifrsig]}
                '';
              }
            ];
          })
        ];
        TaskCompleted = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifrtho];
          })
        ];
        StopFailure = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifdngr kit.sounds.ifrsis];
          })
        ];
        PermissionDenied = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifdngr kit.sounds.permission-denied];
          })
        ];
        PermissionRequest = [
          (kit.mkNotificationEntry {
            image = kit.images.claude;
            title = "Claude Code";
            message = "Permission required";
            extraHooks = [
              (kit.mkAudioHook [kit.sounds.ifdngr kit.sounds.permission-required])
            ];
          })
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
          (kit.mkAudioEntry {
            files = [kit.sounds.ifdarm];
          })
        ];
      };

      companyAnnouncements = [
        "Reminder: you're in solo mode"
      ];

      includeCoAuthoredBy = false;

      permissions = kit.mkAgentPermissions "claude";
    };
  };
}

{
  pkgs,
  config,
  ...
}: {
  programs = {
    gemini-cli = {
      enable = true;
      defaultModel = "gemini-2.5-flash";
      settings = {
        theme = "Default";
        vimMode = true;
        preferredEditor = "nano";
        autoAccept = false;
        secureModeEnabled = false;
        security.auth.selectedType = "oauth-personal";
      };
    };

    claude-code = {
      enable = true;
      enableMcpIntegration = true;
      mcpServers = builtins.removeAttrs config.programs.mcp.servers [
        "expo"
        "supabase"
      ];

      # https://code.claude.com/docs/en/settings#available-settings
      settings = {
        model = "claude-haiku-4-5-20251001";
        effortLevel = "medium";
        outputStyle = "Explanatory";
        language = "english";
        cleanupPeriodDays = 20;

        env = {
          "CLAUDE_CODE_ENABLE_TELEMETRY" = "0";
        };

        hooks = let
          mkAudioHook = mp3: {
            type = "command";
            command = "${pkgs.pulseaudio}/bin/paplay ${mp3}";
          };
        in {
          SessionStart = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifarm.wav)
              ];
            }
          ];
          Elicitation = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/iflmsn.wav)
              ];
            }
          ];
          StopFailure = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifdngr.wav)
                (mkAudioHook ./sounds/ifdarm.wav)
              ];
            }
          ];
          TaskCompleted = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifgood.wav)
                (mkAudioHook ./sounds/ifrtho.wav)
              ];
            }
          ];
          PermissionDenied = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifdngr.wav)
                (mkAudioHook ./sounds/permission-denied.mp3)
              ];
            }
          ];
        };

        companyAnnouncements = [
          "Reminder: you're in solo mode"
        ];

        includeCoAuthoredBy = false;

        memory.text = ''
          No emojis in dev output.
          Use conventional commits in new projects; follow existing conventions otherwise.
          "style" is only for formatting.
        '';

        permissions = {
          allow = [
            "Bash(git diff:*)"
            "Bash(git status:*)"
          ];

          ask = [
            "Bash(git push:*)"
          ];

          defaultMode = "acceptEdits";
          disableBypassPermissionsMode = "disable";
        };
      };
    };
  };
}

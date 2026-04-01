{
  config,
  pkgs,
  ...
}: {
  home.packages = [pkgs.rtk];

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
        "supabase"
      ];

      hooks = let
        mkRemoteScript = (
          {
            url,
            sha256,
          }:
            builtins.readFile (pkgs.fetchurl {inherit url sha256;})
        );
      in {
        "rtk-rewrite.sh" = mkRemoteScript {
          url = "https://raw.githubusercontent.com/rtk-ai/rtk/d6425c311d89a341902432fb82fdd1f524835b8b/hooks/claude/rtk-rewrite.sh";
          sha256 = "1yqqa099iiyp6i3wvjkbffw3njcw4kb6rdjgp3rgazpxjh4n63gg";
        };
      };

      # https://code.claude.com/docs/en/settings#available-settings
      settings = {
        model = "claude-haiku-4-5-20251001";
        effortLevel = "medium";
        outputStyle = "Explanatory";
        language = "english";
        cleanupPeriodDays = 20;

        env = {
          "CLAUDE_CODE_ENABLE_TELEMETRY" = "0";
          "RTK_TELEMETRY_DISABLED" = "1";
        };

        hooks = let
          mkAudioHook = files: {
            type = "command";
            command = builtins.concatStringsSep " && " (
              map (mp3: "${pkgs.pulseaudio}/bin/paplay ${mp3}") files
            );
          };

          mkAudioEvent = files: [
            {
              matcher = "";
              hooks = [
                (mkAudioHook files)
              ];
            }
          ];
        in {
          SessionStart = [
            (mkAudioEvent [./sounds/ifarm.wav])
          ];
          Elicitation = [
            (mkAudioEvent [./sounds/ifrtho.wav])
          ];
          ElicitationResult = [
            (mkAudioEvent [./sounds/ifrtfy.wav])
          ];
          PostToolUseFailure = [
            (mkAudioEvent [./sounds/ifdngr.wav ./sounds/ifvfrs.wav])
          ];
          UserPromptSubmit = [
            (mkAudioEvent [./sounds/ifrsig.wav])
          ];
          TaskCompleted = [
            (mkAudioEvent [./sounds/ifgood.wav ./sounds/ifrtho.wav])
          ];
          StopFailure = [
            (mkAudioEvent [./sounds/ifdngr.wav ./sounds/ifrsis.wav])
          ];
          PermissionDenied = [
            (mkAudioEvent [./sounds/ifdngr.wav ./sounds/permission-denied.mp3])
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
            (mkAudioEvent [./sounds/ifdarm.wav])
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
          Keep commit bodies concise.
          Use lowercase titles unless required.
          Do not use flake-utils in new nix flakes.
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

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
                (mkAudioHook ./sounds/ifrtho.wav)
              ];
            }
          ];
          ElicitationResult = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifrtfy.wav)
              ];
            }
          ];
          PostToolUseFailure = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifdngr.wav)
                (mkAudioHook ./sounds/ifvfrs.wav)
              ];
            }
          ];
          UserPromptSubmit = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifrsig.wav)
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
          StopFailure = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifdngr.wav)
                (mkAudioHook ./sounds/ifrsis.wav)
              ];
            }
          ];
          PermissionDenied = [
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifdngr.wav) # TODO: both should be managed by the script
                (mkAudioHook ./sounds/permission-denied.mp3)
              ];
            }
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
            {
              matcher = "";
              hooks = [
                (mkAudioHook ./sounds/ifdarm.wav)
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

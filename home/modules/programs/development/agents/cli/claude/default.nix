{
  config,
  pkgs,
  lib,
  ...
}: let
  assets = import ../../assets;
  inherit (assets) sounds images;
in {
  programs.claude-code = {
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
        url = "https://raw.githubusercontent.com/rtk-ai/rtk/d6425c311d89a341902432fb82fdd1f524835b8b/hooks/rtk-rewrite.sh";
        sha256 = "sha256-7w1jCZT9fvXyuE+2bNYknEk7uHNrys1HNNfHmBJQGPs=";
      };
    };

    marketplaces = {
      claude-plugins-official = pkgs.fetchFromGitHub {
        owner = "anthropics";
        repo = "claude-plugins-official";
        rev = "b091cb4179d3b62a6e2a39910461c7ec7165b1ef";
        sha256 = "sha256-uKDVcw6C1uzpiIY+hjgHxr4AU9wM1KF7t3v6zd9XBHk=";
      };
    };

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
        "CLAUDE_CODE_ENABLE_TELEMETRY" = "0";
      };

      hooks = let
        playAudio = files: {
          type = "command";
          command = builtins.concatStringsSep " && " (
            map (mp3: "${pkgs.pulseaudio}/bin/paplay ${mp3}") files
          );
        };

        mkAudioEvent = files: {
          matcher = "";
          hooks = [
            (playAudio files)
          ];
        };

        mkNotification = title: message: {
          matcher = "";
          hooks = [
            {
              type = "command";
              command = ''${lib.getExe pkgs.libnotify} -a "${title}" -i "${images.claude}" "${title}" "${message}"'';
            }
          ];
        };
      in {
        Notification = [
          (mkNotification "Claude Code" "Awaiting your input")
        ];
        SessionStart = [
          (mkAudioEvent [sounds.ifarm])
        ];
        Elicitation = [
          (mkAudioEvent [sounds.ifrtho])
        ];
        ElicitationResult = [
          (mkAudioEvent [sounds.ifrtfy])
        ];
        PostToolUseFailure = [
          (mkAudioEvent [sounds.ifdngr sounds.ifvfrs])
        ];
        UserPromptSubmit = [
          (mkAudioEvent [sounds.ifrsig])
        ];
        TaskCompleted = [
          (mkAudioEvent [sounds.ifgood sounds.ifrtho])
        ];
        StopFailure = [
          (mkAudioEvent [sounds.ifdngr sounds.ifrsis])
        ];
        PermissionDenied = [
          (mkAudioEvent [sounds.ifdngr sounds.permission-denied])
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
          (mkAudioEvent [sounds.ifdarm])
        ];
      };

      companyAnnouncements = [
        "Reminder: you're in solo mode"
      ];

      includeCoAuthoredBy = false;

      memory.text = ''
        No emojis in dev output.
        Look for genuine technical dialogue, not validation.
        Use conventional commits in new projects; follow existing conventions otherwise.
        "style" is only for formatting.
        Keep commit bodies concise.
        Use lowercase titles unless required.
        Do not use flake-utils in new nix flakes.
        Avoid git add (-A|.|--all).
        Ensure commit messages match the staged changes.
      '';

      permissionProfile = "standard";
      permissions = {
        defaultMode = pkgs.lib.mkForce "acceptEdits";
        disableBypassPermissionsMode = "disable";
      };
    };
  };
}

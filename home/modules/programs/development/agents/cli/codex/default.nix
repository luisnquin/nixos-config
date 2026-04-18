{
  config,
  agent,
  pkgs,
  lib,
  ...
}: let
  mkAudioHook = files: [
    {
      type = "command";
      command = builtins.concatStringsSep " && " (
        map (mp3: "${pkgs.pulseaudio}/bin/paplay ${mp3}") files
      );
    }
  ];

  mkNotificationHook = image: title: message: [
    {
      type = "command";
      command = ''${lib.getExe pkgs.libnotify} -a "${title}" -i "${image}" "${title}" "${message}"'';
    }
  ];

  inherit (agent.assets) sounds images;
in {
  home.file = {
    ".codex/hooks.json" = {
      source = (pkgs.formats.json {}).generate "codex-hooks" {
        hooks = {
          SessionStart = [
            {
              matcher = "startup";
              hooks = mkAudioHook [sounds.ifarm];
            }
          ];
          PreToolUse = [
            {
              matcher = "Bash";
              hooks = mkAudioHook [sounds.ifrtho];
            }
          ];
          PostToolUse = [
            {
              matcher = "Bash";
              hooks = mkAudioHook [sounds.ifrtfy];
            }
          ];
          UserPromptSubmit = [
            {
              hooks = mkAudioHook [sounds.ifrsig];
            }
          ];
          Stop = [
            {
              hooks = mkAudioHook [sounds.ifdarm];
            }
          ];
          PermissionRequest = [
            {
              hooks = (mkNotificationHook images.codex "Codex" "Permission required") ++ mkAudioHook [sounds.ifdngr sounds.permission-required];
            }
          ];
        };
      };
    };
  };

  programs.codex = {
    enable = true;
    package = pkgs.llm-agents.codex;
    enableMcpIntegration = true;

    custom-instructions = ''
      ${agent.memories}

      ${builtins.readFile "${pkgs.rtk}/share/rtk/hooks/codex/rtk-awareness.md"}
    '';

    settings = {
      analytics.enabled = true;
      feedback.enabled = true;
      mcp_servers = builtins.removeAttrs config.programs.mcp.servers [
        "supabase"
      ];

      agents = {
        job_max_runtime_seconds = 3600;
        max_depth = 5;
        max_threads = 10;
      };

      approval_policy = "untrusted";
      approvals_reviewer = "user";
      sandbox_mode = "workspace-write";

      profiles = {
        coding = {
          personality = "pragmatic";
          features = {
            code_mode = true;
            apply_patch_freeform = true;
          };
        };
        creative = {
          personality = "friendly";
          model_verbosity = "high";
        };
      };

      features = {
        codex_hooks = true;
      };

      tools = {
        view_image = true;
        web_search = {
          context_size = "medium";
          allowed_domains = agent.domains;
        };
      };
    };
  };
}

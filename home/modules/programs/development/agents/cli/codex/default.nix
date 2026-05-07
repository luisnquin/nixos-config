{
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
  permissions = kit.mkAgentPermissions "codex";
in {
  home.file = {
    ".codex/hooks.json" = {
      source = (pkgs.formats.json {}).generate "codex-hooks" {
        hooks = {
          SessionStart = [
            (kit.mkAudioEntry {
              matcher = "startup";
              files = [kit.sounds.ifdarm];
            })
          ];
          PreToolUse = [
            (kit.mkAudioEntry {
              matcher = "Bash";
              files = [kit.sounds.ifrtho];
            })
          ];
          PostToolUse = [
            (kit.mkAudioEntry {
              matcher = "Bash";
              files = [kit.sounds.ifrtfy];
            })
          ];
          UserPromptSubmit = [
            (kit.mkAudioEntry {
              files = [kit.sounds.ifrsig];
            })
          ];
          Stop = [
            (kit.mkAudioEntry {
              files = [kit.sounds.ifdarm];
            })
          ];
          PermissionRequest = [
            (kit.mkNotificationEntry {
              image = kit.images.codex;
              title = "Codex";
              message = "Permission required";
              extraHooks = [
                (kit.mkAudioHook [kit.sounds.ifdngr kit.sounds.permission-required])
              ];
            })
          ];
        };
      };
    };
  };

  programs.codex = {
    enable = true;
    package = pkgs.llm-agents.codex;
    enableMcpIntegration = true;

    context = ''
      ${kit.memories}

      ${builtins.readFile "${pkgs.rtk}/share/rtk/hooks/codex/rtk-awareness.md"}
    '';

    settings = {
      analytics.enabled = true;
      feedback.enabled = true;
      mcp_servers = kit.mkMcpServers {
        snakeCase = true;
      };

      agents = {
        job_max_runtime_seconds = 3600;
        max_depth = 5;
        max_threads = 10;
      };

      approval_policy = permissions.approval_policy;
      sandbox_mode = permissions.sandbox_mode;
      approvals_reviewer = "user";

      shell_environment_policy = {
        "inherit" = "core";
        set = {
          CODEX_AGENT = "1";
          PINENTRY_USER_DATA = "gui";
        };
      };

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
        web_search = permissions.web_search;
      };
    };
  };
}

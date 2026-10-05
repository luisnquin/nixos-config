{
  mkAgentKit,
  config,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
  permissions = kit.mkAgentPermissions "codex" {};
in {
  imports = [
    ./hooks.nix
  ];

  programs.codex = {
    enable = true;
    package = pkgs.llm-agents.codex;
    enableMcpIntegration = true;

    context = ''
      ${kit.memories}

      Devices go through `phone`, never raw adb, simctl or emulator. Each thread holds its own device; to hand one to a sub-agent, run `phone release -t <device>` first and let the sub-agent acquire it.

      ${builtins.readFile "${pkgs.rtk}/share/rtk/hooks/rtk-awareness-high.md"}
    '';

    settings = {
      model = "gpt-6-sol";
      model_reasoning_effort = "medium";

      analytics.enabled = true;
      feedback.enabled = true;
      mcp_servers =
        (kit.mkMcpServers {
          snakeCase = true;
        })
        // {
          sponsorbar.url = "https://sponsorbar.io/mcp";
        };

      agents = {
        job_max_runtime_seconds = 3600;
        max_depth = 5;
        max_threads = 10;
      };

      sandbox_mode = permissions.sandbox_mode;
      approvals_reviewer = "user";

      shell_environment_policy = {
        "inherit" = "all";
        ignore_default_excludes = false;
        include_only = [
          "PATH"
          "SHELL"
          "TMPDIR"
          "TEMP"
          "TMP"
          "HOME"
          "LANG"
          "LC_ALL"
          "LC_CTYPE"
          "LOGNAME"
          "USER"
          "HERDR_*"
          "CODEX_AGENT"
          "CODEX_THREAD_ID"
          "CODEX_SESSION_ID"
          "GIT_ASKPASS"
          "GIT_SSH_COMMAND"
          "GIT_TERMINAL_PROMPT"
          "PINENTRY_USER_DATA"
          "SSH_ASKPASS"
          "SSH_AUTH_SOCK"
        ];
        set = {
          CODEX_AGENT = "1";
          GIT_ASKPASS = "${pkgs.coreutils}/bin/false";
          GIT_SSH_COMMAND = "ssh -o BatchMode=yes -o IdentityAgent=none";
          GIT_TERMINAL_PROMPT = "0";
          PINENTRY_USER_DATA = "gui";
          SSH_ASKPASS = "${pkgs.coreutils}/bin/false";
          SSH_AUTH_SOCK = "";
        };
      };

      projects = let
        trustAll = paths:
          pkgs.lib.genAttrs (map (path: "${config.home.homeDirectory}/${path}") paths) (_: {
            trust_level = "trusted";
          });
      in
        trustAll [
          ".dotfiles"
          "Projects/github.com/luisnquin"
          "Projects/github.com/cuentacero"
          "Projects/github.com/0xc000022070"
        ];

      tui = {
        show_tooltips = false;
        status_line = [
          "current-dir"
          "model"
          "reasoning"
          "branch-changes"
          "context-used"
          "five-hour-limit"
          "weekly-limit"
        ];
      };

      features = {
        hooks = true;
        context_management.experimental_mode = true;
      };

      tools = {
        view_image = true;
        web_search = permissions.web_search;
      };
    };

    profiles = {
      coding = {
        features = {
          code_mode = true;
          apply_patch_freeform = true;
        };
      };
      creative = {
        model_verbosity = "high";
      };
    };
  };
}

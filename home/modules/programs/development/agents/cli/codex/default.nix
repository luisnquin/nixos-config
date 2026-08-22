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

      ${builtins.readFile "${pkgs.rtk}/share/rtk/hooks/codex/rtk-awareness.md"}
    '';

    settings = {
      model = "gpt-5.6-sol";
      model_reasoning_effort = "medium";

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
        model_availability_nux = {
          "gpt-5.5" = 1;
        };
      };

      features = {
        hooks = true;
      };

      tools = {
        view_image = true;
        web_search = permissions.web_search;
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
  };
}

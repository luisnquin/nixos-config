{
  mkAgentKit,
  config,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
  permissions = kit.mkAgentPermissions "codex" {};

  codex = pkgs.llm-agents.codex.overrideAttrs (old: {
    patches = (old.patches or []) ++ [
      ./recursive-project-trust.patch
      ./git-remote-show-no-fetch.patch
    ];
  });
in {
  imports = [
    ./hooks.nix
  ];

  programs.codex = {
    enable = true;
    package = codex;
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

      tui.model_availability_nux = {
        "gpt-5.5" = 1;
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
        hooks = true;
      };

      tools = {
        view_image = true;
        web_search = permissions.web_search;
      };
    };
  };
}

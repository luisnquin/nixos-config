{
  mkAgentKit,
  config,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
  permissions = kit.mkAgentPermissions "codex" {};

  codex = pkgs.llm-agents.codex.overrideAttrs (old: let
    devServerInstruction = "When building a site or app that needs a dev server to run properly, you start the local dev server after implementation and give the user the URL so they can try it. If there's already a server on that port, you use another one. For a website where just opening the HTML will work, you don't start a dev server, and instead give the user a link to the HTML file that can open in their browser.\\n\\n";
  in {
    patches =
      (old.patches or [])
      ++ [
        ./recursive-project-trust.patch
        ./git-remote-show-no-fetch.patch
        ./curated-plugins-disable-sync.patch
        ./presentation-card.patch
        ./terminal-terminfo-dirs.patch
        ./custom-input-bar.patch
        ./disable-cloud-tasks.patch
      ];

    postPatch =
      (old.postPatch or "")
      + ''
        substituteInPlace models-manager/models.json \
          --replace-fail ${pkgs.lib.escapeShellArg devServerInstruction} ""
      '';
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

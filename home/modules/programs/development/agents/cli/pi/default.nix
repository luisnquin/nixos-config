{
  mkAgentKit,
  config,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};

  configDir = "${config.home.homeDirectory}/.pi/agent";

  # pi has no allow/deny tool-permission model; its only gate is per-project
  # trust. Mirror the kit's autonomous switch used by the other agents.
  isAutonomous = (config.agents.permissions.profile or "standard") == "autonomous";

  commitPrompt = ''
    ---
    description: Atomic conventional commit from current changes
    argument-hint: "[optional-scope]"
    ---

    ## Context
    Run in the repo root and use the output of:
    - `git status --short --branch`
    - `git diff --staged --stat` and `git diff --staged`
    - `git diff --stat` and `git diff`
    - `git log --oneline -5`
    - `git config --file .gitmodules --get-regexp path >/dev/null 2>&1 && git submodule status --recursive || echo no submodules`

    ## Task
    If there are no staged or unstaged changes, stop and report "Nothing to commit."

    Otherwise, create one or more atomic git commits:

    Requirements:
    - Conventional commit format: `<type>(<scope>): <description>`
    - Valid types: feat, fix, refactor, chore, docs, test, style, perf, ci, build
    - Stage only files that belong to the same logical change
    - If changes are unrelated, create separate commits for each coherent subset
    - Body only when it adds non-obvious context (why, not what)
    - Max 72 chars in subject line
    - Present tense, imperative mood ("add" not "added")
  '';
in {
  # https://pi.dev/docs/latest/prompt-templates
  home.file."${configDir}/prompts/commit.md".text = commitPrompt;

  programs.pi-coding-agent = {
    enable = true;
    package = pkgs.llm-agents.pi;

    # https://pi.dev/docs/latest/settings
    settings = {
      defaultProvider = "openai";
      defaultModel = "gpt-5.5";
      defaultThinkingLevel = "medium";

      theme = "dark";
      externalEditor = "nano";
      quietStartup = true;

      enableInstallTelemetry = false;
      enableAnalytics = false;

      defaultProjectTrust =
        if isAutonomous
        then "trusted"
        else "ask";

      enabledModels = [
        "gpt-5.5*"
        "gpt-5.4*"
        "claude-opus-*"
        "claude-sonnet-*"
        "qwen*"
        "gemma*"
      ];

      compaction = {
        enabled = true;
        reserveTokens = 16384;
        keepRecentTokens = 20000;
      };

      retry = {
        enabled = true;
        maxRetries = 3;
      };

      # https://pi.dev/docs/latest/extensions
      # rtk rewrites bash commands to token-optimized equivalents.
      extensions = [
        "${pkgs.rtk}/share/rtk/hooks/pi/rtk.ts"
      ];
    };

    # https://pi.dev/docs/latest/models
    models = {
      providers.litellm = {
        baseUrl = "http://rose.local:4000/v1";
        api = "openai-completions";
        apiKey = "dummy";
        models = [
          {
            id = "qwen2.5-coder:7b";
            name = "Qwen 2.5 - Coder";
          }
          {
            id = "gemma4:e4b";
            name = "Gemma 4";
          }
        ];
      };
    };

    context = kit.memories;
  };
}

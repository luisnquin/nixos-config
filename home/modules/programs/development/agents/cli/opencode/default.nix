{
  pkgs,
  mkAgentKit,
  ...
}: let
  kit = mkAgentKit {};
in {
  # https://github.com/anomalyco/opencode/blob/dev/packages/web/src/content/docs/commands.mdx
  xdg.configFile."opencode/commands/commit.md".text = ''
    ---
    description: Atomic conventional commit from current changes
    argument-hint: "[optional-scope]"
    ---

    ## Context
    - Branch and status:
      !`git status --short --branch`
    - Staged changes:
      !`git diff --staged --stat && git diff --staged`
    - Unstaged changes:
      !`git diff --stat && git diff`
    - Recent commits (for style reference):
      !`git log --oneline -5`
    - Submodules:
      !`git config --file .gitmodules --get-regexp path >/dev/null 2>&1 && git submodule status --recursive || echo "no submodules"`

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

  # https://opencode.ai/config.json
  programs.opencode = {
    enable = true;
    package = pkgs.llm-agents.opencode;

    plugins = {
      "rtk" = "${pkgs.rtk}/share/rtk/hooks/opencode-rtk.ts";
    };

    settings = {
      server.port = 4096;

      permission = kit.mkAgentPermissions "opencode";

      model = "litellm/qwen2.5-coder:7b";

      provider = {
        litellm = {
          npm = "@ai-sdk/openai-compatible";
          name = "LiteLLM";
          options = {
            "baseURL" = "http://rose.local:4000/v1";
            "apiKey" = "dummy";
          };
          models = {
            "gemma4:e4b" = {
              "name" = "Gemma 4";
            };
            "qwen2.5-coder:7b" = {
              "name" = "Qwen 2.5 - Coder";
            };
          };
        };
      };

      snapshot = true;
      autoupdate = false;
    };
  };
}

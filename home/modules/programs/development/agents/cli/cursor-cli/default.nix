{
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
in {
  imports = [
    ./hooks.nix
  ];

  programs.cursor-agent = {
    enable = true;
    package = pkgs.symlinkJoin {
      name = "cursor-agent";
      paths = [pkgs.cursor-cli];
      buildInputs = [pkgs.makeWrapper];

      postBuild = ''
        wrapProgram "$out/bin/cursor-agent" \
          --set PINENTRY_USER_DATA gui

        ln -s cursor-agent "$out/bin/agent"
      '';
    };
    enableMcpIntegration = true;
    rules = {
      "global.mdc" = ''
        ---
        alwaysApply: true
        ---
        ${kit.memories}
      '';
    };

    # https://cursor.com/docs/reference/plugins (command markdown + frontmatter)
    commands."commit.md" = ''
      ---
      name: commit
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
    settings = {
      attribution = {
        attributeCommitsToAgent = false;
        attributePRsToAgent = false;
      };
    };
  };
}

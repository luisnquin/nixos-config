{
  mkAgentKit,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};

  rtkPath = ".gemini/hooks/rtk-hook-gemini.sh";

  antigravityPermissions = kit.mkAgentPermissions "antigravity" {};

  commitPrompt = ''
    ## Context
    - Branch and status:
    !{git status --short --branch}
    - Staged stat:
    !{git diff --staged --stat}
    - Staged patch:
    !{git diff --staged}
    - Unstaged stat:
    !{git diff --stat}
    - Unstaged patch:
    !{git diff}
    - Recent commits (style reference):
    !{git log --oneline -5}
    - Submodules:
    !{sh -c "git config --file .gitmodules --get-regexp path >/dev/null 2>&1 && git submodule status --recursive || echo no submodules"}

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
  imports = [
    ./hooks.nix
  ];

  home.file = {
    "${rtkPath}" = {
      text = ''
        #!/bin/sh
        exec rtk hook gemini
      '';
      executable = true;
    };

    ".gemini/.env".text = ''
      PINENTRY_USER_DATA=gui
    '';
  };

  programs.antigravity-cli = {
    enable = true;
    package = lib.lowPrio pkgs.llm-agents.antigravity-cli;
    enableMcpIntegration = true;
    defaultModel = "gemini-2.5-flash";

    context = {
      GEMINI = kit.memories;
    };

    permissions = {
      allow = antigravityPermissions.allow;
      ask = antigravityPermissions.ask;
      deny = antigravityPermissions.deny;
    };

    commands.commit = {
      description = "Atomic conventional commit from current changes";
      prompt = commitPrompt;
    };

    settings = {
      vimMode = true;
      preferredEditor = "nano";
    };
  };
}

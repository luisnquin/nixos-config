{
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};

  rtkPath = ".gemini/hooks/rtk-hook-gemini.sh";

  geminiTools = kit.mkAgentPermissions "gemini";

  mkPolicy = decision: rule: let
    match = builtins.match "([A-Za-z_]+)\\((.*)\\)" rule;
    toolName =
      if match != null
      then builtins.elemAt match 0
      else rule;
    args =
      if match != null
      then builtins.elemAt match 1
      else "*";
    hasArgs = args != "*";
    prefixRaw = pkgs.lib.removeSuffix "*" args;
  in
    ''
      [[rule]]
      toolName = "${toolName}"
      decision = "${
        if decision == "ask"
        then "ask_user"
        else decision
      }"
      priority = ${
        if decision == "deny"
        then "200"
        else "100"
      }
    ''
    + pkgs.lib.optionalString (hasArgs && toolName == "run_shell_command") ''
      commandPrefix = "${prefixRaw}"
    '';

  policyToml = pkgs.lib.concatStringsSep "\n" (
    (map (mkPolicy "allow") geminiTools.allowed)
    ++ (map (mkPolicy "ask") geminiTools.confirmationRequired)
    ++ (map (mkPolicy "deny") geminiTools.exclude)
  );
in {
  home.file = {
    # https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/custom-commands.md
    ".gemini/commands/commit.toml".text = ''
      description = "Atomic conventional commit from current changes"

      prompt = """
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
      """
    '';

    "${rtkPath}" = {
      text = ''
        #!/bin/sh
        exec rtk hook gemini
      '';
      executable = true;
    };
    ".gemini/policies/nix-rules.toml".text = policyToml;

    ".gemini/.env".text = ''
      PINENTRY_USER_DATA=gui
    '';
  };

  programs.gemini-cli = {
    enable = true;
    package = pkgs.llm-agents.gemini-cli;
    defaultModel = "gemini-2.5-flash";

    context = {
      GEMINI = kit.memories;
    };

    settings = {
      theme = "Default";
      vimMode = true;
      preferredEditor = "nano";
      autoAccept = false;
      secureModeEnabled = false;
      security.auth.selectedType = "oauth-personal";
    };
  };
}

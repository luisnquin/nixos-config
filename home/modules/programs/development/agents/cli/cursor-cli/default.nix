{
  mkAgentKit,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};
  jq = lib.getExe pkgs.jq;
  # Not in kit.images: generated set can skip files depending on flake source filter.

  mkHookScript = body: ''
    #!/usr/bin/env bash
    set -euo pipefail
    cat >/dev/null
    ${body}
    echo '{}'
  '';

  mkInputHookScript = body: ''
    #!/usr/bin/env bash
    set -euo pipefail
    input=$(cat)
    ${body}
    echo '{}'
  '';
in {
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

    hookScripts = {
      "audit.sh" = ''
        #!/usr/bin/env bash
        set -euo pipefail
        json_input=$(cat)
        timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        log_file=''${XDG_STATE_HOME:-$HOME/.local/state}/cursor/hooks-audit.log
        mkdir -p "$(dirname "$log_file")"
        echo "[$timestamp] $json_input" >> "$log_file"
        exit 0
      '';

      "session-start-audio.sh" =
        mkHookScript (kit.mkAudioCmd [kit.sounds.ifarm]);

      "session-end-audio.sh" =
        mkHookScript (kit.mkAudioCmd [kit.sounds.ifdarm]);

      "submit-prompt-audio.sh" = mkHookScript ''
        ${kit.mkCancelNotificationCmd {
          sequenceId = "cursor-awaiting-input";
        }}

        ${kit.mkAudioCmd [kit.sounds.ifrsig]}
      '';

      "subagent-start-audio.sh" =
        mkHookScript (kit.mkAudioCmd [kit.sounds.ifrtho]);

      "subagent-stop-audio.sh" =
        mkHookScript (kit.mkAudioCmd [kit.sounds.ifrtfy]);

      "post-tool-use-failure.sh" = mkInputHookScript ''
        ft=$(${jq} -r '.failure_type // empty' <<<"$input")
        if [[ "$ft" == "permission_denied" ]]; then
          ${kit.mkAudioCmd [
          kit.sounds.ifdngr
          kit.sounds."permission-denied"
        ]}
        else
          ${kit.mkAudioCmd [kit.sounds.ifvfrs]}
        fi
      '';

      "stop.sh" = mkInputHookScript ''
        st=$(${jq} -r '.status // empty' <<<"$input")
        if [[ "$st" == "error" ]]; then
          ${kit.mkAudioCmd [
          kit.sounds.ifdngr
          kit.sounds.ifrsis
        ]}
        fi
      '';

      "awaiting-input-notify.sh" = mkHookScript (
        (kit.mkNotificationCmd kit.images.cursor "Cursor" "Awaiting your input" {
          ntfy = {
            delay = "10s";
            sequenceId = "cursor-awaiting-input";
          };
        })
        + " && "
        + (kit.mkAudioCmd [kit.sounds.buzact])
      );
    };

    hookEvents = {
      sessionStart = [
        {command = "./hooks/audit.sh";}
        {command = "./hooks/session-start-audio.sh";}
      ];
      sessionEnd = [
        {command = "./hooks/audit.sh";}
        {command = "./hooks/session-end-audio.sh";}
      ];
      beforeSubmitPrompt = [
        {
          matcher = "UserPromptSubmit";
          command = "./hooks/submit-prompt-audio.sh";
        }
      ];
      postToolUseFailure = [
        {command = "./hooks/post-tool-use-failure.sh";}
      ];
      stop = [
        {command = "./hooks/stop.sh";}
      ];
      subagentStart = [
        {command = "./hooks/subagent-start-audio.sh";}
      ];
      subagentStop = [
        {command = "./hooks/subagent-stop-audio.sh";}
      ];
      afterAgentResponse = [
        {
          matcher = "AgentResponse";
          command = "./hooks/awaiting-input-notify.sh";
        }
      ];
    };
  };
}

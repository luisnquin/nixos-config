{
  config,
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
in {
  home.file.".claude/commands/commit.md".text = ''
    ---
    allowed-tools: Bash(git status:*), Bash(git diff:*), Bash(git log:*), Bash(git config:*), Bash(git submodule:*), Bash(git add:*), Bash(git commit:*)
    description: Atomic conventional commit from current changes
    argument-hint: [optional-scope]
    model: haiku
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

  programs.claude-code = {
    enable = true;
    package = pkgs.llm-agents.claude-code;
    enableMcpIntegration = true;
    mcpServers = kit.mkMcpServers {};

    hooks = {
      "rtk-rewrite.sh" = builtins.readFile "${pkgs.rtk}/share/rtk/hooks/claude/rtk-rewrite.sh";
    };

    marketplaces = {
      claude-plugins-official = pkgs.fetchFromGitHub {
        owner = "anthropics";
        repo = "claude-plugins-official";
        rev = "b091cb4179d3b62a6e2a39910461c7ec7165b1ef";
        sha256 = "sha256-uKDVcw6C1uzpiIY+hjgHxr4AU9wM1KF7t3v6zd9XBHk=";
      };
    };

    context = kit.memories;

    # https://code.claude.com/docs/en/settings#available-settings
    settings = {
      enabledPlugins = {
        "rust-analyzer-lsp@claude-plugins-official" = true;
      };

      model = "claude-haiku-4-5-20251001";
      effortLevel = "medium";
      outputStyle = "Explanatory";
      language = "english";
      cleanupPeriodDays = 20;

      env = {
        "CLAUDE_CODE_AUTO_COMPACT_WINDOW" = "200000";
        "CLAUDE_CODE_ENABLE_TELEMETRY" = "0";
        "DISABLE_AUTOUPDATER" = "1";
        "PINENTRY_USER_DATA" = "gui";
      };
      # commands = rec {
      #   commit = ''

      #   '';
      #   ac = commit;
      # };

      hooks = {
        Notification = [
          (kit.mkCmdEntry {
            commands = [
              (kit.mkNotificationCmd kit.images.claude "Claude Code" "Awaiting your input" {
                ntfy = {
                  delay = "10s";
                  sequenceId = "claude-awaiting-input";
                };
              })
              (kit.mkAudioCmd [kit.sounds.buzact])
            ];
          })
        ];
        SessionStart = [
          (kit.mkCmdEntry {
            commands = [(kit.mkAudioCmd [kit.sounds.ifarm])];
          })
        ];
        Elicitation = [
          (kit.mkCmdEntry {
            commands = [(kit.mkAudioCmd [kit.sounds.ifrtho])];
          })
        ];
        ElicitationResult = [
          (kit.mkCmdEntry {
            commands = [
              (kit.mkCancelNotificationCmd {sequenceId = "claude-awaiting-input";})
              (kit.mkAudioCmd [kit.sounds.ifrtfy])
            ];
          })
        ];
        PostToolUseFailure = [
          (kit.mkCmdEntry {
            commands = [(kit.mkAudioCmd [kit.sounds.ifvfrs])];
          })
        ];
        UserPromptSubmit = [
          (kit.mkCmdEntry {
            commands = [
              (kit.mkCancelNotificationCmd {sequenceId = "claude-awaiting-input";})
              (kit.mkAudioCmd [kit.sounds.ifrsig])
            ];
          })
        ];
        TaskCompleted = [
          (kit.mkCmdEntry {
            commands = [(kit.mkAudioCmd [kit.sounds.ifrtho])];
          })
        ];
        StopFailure = [
          (kit.mkCmdEntry {
            commands = [(kit.mkAudioCmd [kit.sounds.ifdngr kit.sounds.ifrsis])];
          })
        ];
        PermissionDenied = [
          (kit.mkCmdEntry {
            commands = [(kit.mkAudioCmd [kit.sounds.ifdngr kit.sounds.permission-denied])];
          })
        ];
        PermissionRequest = [
          (kit.mkCmdEntry {
            commands = [
              (kit.mkNotificationCmd kit.images.claude "Claude Code" "Permission required" {})
              (kit.mkAudioCmd [kit.sounds.ifdngr kit.sounds.permission-required])
            ];
          })
        ];
        PreToolUse = [
          (kit.mkCmdEntry {
            matcher = "Bash";
            commands = [config.programs.claude-code.hooks."rtk-rewrite.sh"];
          })
        ];
        SessionEnd = [
          (kit.mkCmdEntry {
            commands = [(kit.mkAudioCmd [kit.sounds.ifdarm])];
          })
        ];
      };

      companyAnnouncements = [
        "Reminder: you're in solo mode"
      ];

      includeCoAuthoredBy = false;

      permissions = kit.mkAgentPermissions "claude";
    };
  };
}

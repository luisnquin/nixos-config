{
  mkAgentKit,
  config,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};
  cbm = lib.getExe pkgs.codebase-memory-mcp;
  phone = lib.getExe pkgs.phone;
  redactSecrets = lib.getExe pkgs.redact-secrets;
  herdrSession = kit.mkHerdrSessionCmd "claude";
  diffReview = pkgs.writeShellApplication {
    name = "agent-diff-review";
    runtimeInputs = [pkgs.git pkgs.jq pkgs.coreutils pkgs.gawk pkgs.gnugrep];
    text = builtins.readFile ../../hooks/diff-review.sh;
  };
  emailGuard = pkgs.writeShellApplication {
    name = "agent-email-guard";
    runtimeInputs = [pkgs.jq pkgs.gnugrep];
    text = builtins.readFile ../../hooks/email-guard.sh;
  };
  rmGuard = pkgs.writeShellApplication {
    name = "agent-rm-guard";
    runtimeInputs = [pkgs.jq pkgs.gnugrep];
    text = builtins.readFile ../../hooks/rm-guard.sh;
  };
  searchScopeGuard = pkgs.writeShellApplication {
    name = "agent-search-scope-guard";
    runtimeInputs = [pkgs.jq pkgs.coreutils];
    text = builtins.readFile ../../hooks/search-scope-guard.sh;
  };
  commandRules = pkgs.writeShellApplication {
    name = "agent-command-rules";
    runtimeInputs = [pkgs.jq];
    runtimeEnv.AGENT_COMMAND_RULES = pkgs.writeText "agent-command-rules.json" (builtins.toJSON [
      {
        pattern = "(^|[;&|({[:space:]])(until|while)[[:space:]].*sleep[[:space:]]|for[[:space:]].*\\$\\(seq[[:space:]].*sleep[[:space:]]";
        foreground = true;
        reason = "Foreground polling loop refused: it shows no progress until it ends, so the user kills it. Run it with run_in_background: true (you are notified when it exits) or use the Monitor tool. For devices, use `phone wait --timeout`.";
      }
      {
        pattern = "(^|[;&|({[:space:]])npx[[:space:]]+convex([[:space:]]|$)";
        reason = "Do not call npx convex. Use the package.json scripts (npm run convex:dev, convex:deploy, convex:codegen); read package.json for the rest.";
      }
    ]);
    text = builtins.readFile ../../hooks/command-rules.sh;
  };
  interruptLog = pkgs.writeShellApplication {
    name = "agent-interrupt-log";
    runtimeInputs = [pkgs.jq pkgs.coreutils pkgs.gnugrep];
    text = builtins.readFile ../../hooks/interrupt-log.sh;
  };
  upgraded = timeout: {
    hooks = [
      {
        type = "command";
        command = "upgraded hook claude";
        inherit timeout;
      }
    ];
  };
  engineHooks =
    lib.genAttrs [
      "SessionStart"
      "UserPromptSubmit"
      "PostToolUse"
      "PostToolUseFailure"
      "Notification"
      "Stop"
      "StopFailure"
      "SessionEnd"
    ] (_: [(upgraded 10)])
    // {PermissionRequest = [(upgraded 90)];};
in {
  programs.claude-code.settings.hooks = lib.zipAttrsWith (_: lib.concatLists) [
    {
      Notification = [
        (kit.mkCmdEntry {
          matcher = "idle_prompt";
          commands = [
            (kit.mkTerminalStatusCmd "waiting" "claude · input")
            (kit.mkNotificationCmd kit.images.claude "Claude Code" "Awaiting your input" {
              context = true;
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
          commands = [
            (kit.mkTerminalStatusCmd "clear" "claude")
            (kit.mkAudioCmd [kit.sounds.ifarm])
            herdrSession
          ];
        })
      ];
      Elicitation = [
        (kit.mkCmdEntry {
          commands = [
            (kit.mkTerminalStatusCmd "waiting" "claude · input")
            (kit.mkAudioCmd [kit.sounds.ifrtho])
          ];
        })
      ];
      ElicitationResult = [
        (kit.mkCmdEntry {
          commands = [
            (kit.mkTerminalStatusCmd "working" "claude · working")
            (kit.mkCancelNotificationCmd {sequenceId = "claude-awaiting-input";})
            (kit.mkAudioCmd [kit.sounds.ifrtfy])
          ];
        })
      ];
      PostToolUse = [
        (kit.mkCmdEntry {
          matcher = "Edit|Write|MultiEdit|NotebookEdit";
          commands = ["${lib.getExe diffReview} 2>/dev/null || true"];
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
            (kit.mkTerminalStatusCmd "working" "claude · working")
            (kit.mkCancelNotificationCmd {sequenceId = "claude-awaiting-input";})
            (kit.mkAudioCmd [kit.sounds.ifrsig])
            "${lib.getExe interruptLog} || true"
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
          commands = [
            (kit.mkTerminalStatusCmd "error" "claude · error")
            (kit.mkAudioCmd [kit.sounds.ifdngr kit.sounds.ifrsis])
          ];
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
            (kit.mkTerminalStatusCmd "waiting" "claude · permission")
            (kit.mkNotificationCmd kit.images.claude "Claude Code" "Permission required" {context = true;})
            (kit.mkAudioCmd [kit.sounds.ifdngr kit.sounds.permission-required])
          ];
        })
      ];
      PreToolUse = [
        (kit.mkCmdEntry {
          commands = [(lib.getExe emailGuard)];
        })
        (kit.mkCmdEntry {
          matcher = "Bash";
          commands = [(lib.getExe rmGuard)];
        })
        (kit.mkCmdEntry {
          matcher = "Bash";
          commands = [(lib.getExe commandRules)];
        })
        (kit.mkCmdEntry {
          matcher = "Bash|Grep";
          commands = [(lib.getExe searchScopeGuard)];
        })
        (kit.mkCmdEntry {
          matcher = "Bash";
          commands = [config.programs.claude-code.hooks."rtk-rewrite.sh"];
        })
        (kit.mkCmdEntry {
          matcher = "Bash";
          commands = ["${phone} hook --harness claude"];
        })
        (kit.mkCmdEntry {
          matcher = "Read";
          commands = ["${redactSecrets} --read-hook"];
        })
        # Injects codebase-memory-mcp graph context into Grep/Glob calls.
        # Never blocks: forced exit 0 even when the project is unindexed.
        (kit.mkCmdEntry {
          matcher = "Grep|Glob";
          commands = ["${cbm} hook-augment 2>/dev/null || true"];
        })
      ];
      SessionEnd = [
        (kit.mkCmdEntry {
          commands = [
            (kit.mkTerminalStatusCmd "clear" "")
            (kit.mkAudioCmd [kit.sounds.ifdarm])
          ];
        })
      ];
      Stop = [
        (kit.mkCmdEntry {
          commands = [(kit.mkTerminalStatusCmd "waiting" "claude · ready")];
        })
      ];
    }
    engineHooks
  ];
}

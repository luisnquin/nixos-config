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
  herdrSession = kit.mkHerdrSessionCmd "claude";
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
          commands = [
            (kit.mkTerminalStatusCmd "waiting" "claude · input")
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
        (kit.mkCmdEntry {
          matcher = "Bash";
          commands = ["${phone} hook --harness claude"];
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

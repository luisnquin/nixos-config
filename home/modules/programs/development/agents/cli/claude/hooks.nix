{
  mkAgentKit,
  config,
  ...
}: let
  kit = mkAgentKit {};
in {
  programs.claude-code.settings.hooks = {
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
}

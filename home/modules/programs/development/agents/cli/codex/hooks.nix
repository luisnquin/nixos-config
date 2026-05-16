{
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
in {
  home.file = {
    ".codex/hooks.json" = {
      source = (pkgs.formats.json {}).generate "codex-hooks" {
        hooks = {
          SessionStart = [
            (kit.mkCmdEntry {
              matcher = "startup";
              commands = [(kit.mkAudioCmd [kit.sounds.ifdarm])];
            })
          ];
          PreToolUse = [
            (kit.mkCmdEntry {
              matcher = "Bash";
              commands = [(kit.mkAudioCmd [kit.sounds.ifrtho])];
            })
          ];
          PostToolUse = [
            (kit.mkCmdEntry {
              matcher = "Bash";
              commands = [(kit.mkAudioCmd [kit.sounds.ifrtfy])];
            })
          ];
          UserPromptSubmit = [
            (kit.mkCmdEntry {
              commands = [(kit.mkAudioCmd [kit.sounds.ifrsig])];
            })
          ];
          Stop = [
            (kit.mkCmdEntry {
              commands = [(kit.mkAudioCmd [kit.sounds.ifdarm])];
            })
          ];
          PermissionRequest = [
            (kit.mkCmdEntry {
              commands = [
                (kit.mkNotificationCmd kit.images.codex "Codex" "Permission required" {})
                (kit.mkAudioCmd [kit.sounds.ifdngr kit.sounds.permission-required])
              ];
            })
          ];
        };
      };
    };
  };
}

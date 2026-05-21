{
  mkAgentKit,
  config,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};
in {
  programs.codex.settings.hooks.state =
    lib.concatMapAttrs (
      hookType: hashes: let
        hooksFile = "${config.home.homeDirectory}/.codex/hooks.json";
        mkHookKey = type: listIdx: cmdIdx: "${hooksFile}:${type}:${toString listIdx}:${toString cmdIdx}";
      in
        lib.listToAttrs (lib.imap0 (
            cmdIdx: hash:
              lib.nameValuePair
              (mkHookKey hookType 0 cmdIdx)
              {trusted_hash = "sha256:${hash}";}
          )
          hashes)
    ) {
      session_start = ["f574f1c60e3409b03f6d22fa466d6c4c8b5d8783e55c7dfe0f5f94e1dd3df5a0"];
      pre_tool_use = ["3e8090a6213fd04f2f71889d0b9111b4efb3879e912ac4b3929b4e71da7b066a"];
      post_tool_use = ["fffe40216f966649dd0558146a5d98948f2c93903862a01b5d4dfa57de068247"];
      user_prompt_submit = ["bcef8000e08ab04a7130c6dde7ea3fbf613595edfcd140f5474d1100bdc5046f"];
      stop = ["d2f96b3e97b03de19e8eb2c5c54ce4f411ab20f485136b5e23183b46f20c3e9c"];
      permission_request = [
        "3526324033b32b0855b136f0e3e64c39446a3ed3a3d27ea04e18698932117c44"
        "dc164b1f0b82ce6f30050ff282e1252420b637fbb93224c6811d61cac9142ccd"
      ];
    };

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

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
      permission_request = [
        "3b7a0db516ccf5e8d3e131909ca0f6fae17a5ae19c2d541399f9a26bf0da3d14"
        "26488b1063551d9525386288cb681b118df136b1e85570b3711ae6b60a9f0e7f"
      ];
      post_tool_use = ["5e1e3e34a75e7747c2a5253ded7fcb0e5013d0d7503ff9969ebece65265bf695"];
      pre_tool_use = ["eb173c9f99fc39499606cf97d06de578695e67bdddd54651836e3efce7298fb6"];
      session_start = ["eb2adfebfb6b4842ebf56365f362a0c39e42c2eb15199bb7aa5d97a9bd87d150"];
      stop = ["48419fe79cf0e81e9efaf50b1981baf3da4de98ce15d15eecdc9d77f5c5cb492"];
      user_prompt_submit = ["da4bfd1c28452c49ad0c46d0cdea3c69b9e05e0be57ee9d394ee497a4c7937aa"];
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

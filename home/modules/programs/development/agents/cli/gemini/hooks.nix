{
  mkAgentKit,
  config,
  ...
}: let
  rtkPath = ".gemini/hooks/rtk-hook-gemini.sh";

  kit = mkAgentKit {};
in {
  programs.gemini-cli.settings.hooks = {
    SessionStart = [
      (kit.mkCmdEntry {
        commands = [(kit.mkAudioCmd [kit.sounds.ifarm])];
      })
    ];

    SessionEnd = [
      (kit.mkCmdEntry {
        commands = [(kit.mkAudioCmd [kit.sounds.ifdarm])];
      })
    ];

    BeforeAgent = [
      (kit.mkCmdEntry {
        commands = [
          (kit.mkCancelNotificationCmd {sequenceId = "gemini-awaiting-input";})
          (kit.mkAudioCmd [kit.sounds.ifrsig])
        ];
      })
    ];

    PreCompress = [
      (kit.mkCmdEntry {
        commands = [(kit.mkAudioCmd [kit.sounds.ifrsig])];
      })
    ];

    Notification = [
      (kit.mkCmdEntry {
        commands = [
          (kit.mkNotificationCmd kit.images.gemini "Gemini" "Awaiting your input" {
            ntfy = {
              delay = "10s";
              sequenceId = "gemini-awaiting-input";
            };
          })
          (kit.mkAudioCmd [kit.sounds.buzact])
        ];
      })
    ];

    BeforeTool = [
      (kit.mkCmdEntry {
        matcher = "run_shell_command";
        commands = ["${config.home.homeDirectory}/${rtkPath}"];
      })
    ];
  };
}

{
  mkAgentKit,
  config,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};

  rtkPath = ".gemini/hooks/rtk-hook-gemini.sh";
in {
  home.file = {
    "${rtkPath}" = {
      text = ''
        #!/bin/sh
        exec rtk hook gemini
      '';
      executable = true;
    };
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

      tools = kit.mkAgentPermissions "gemini";

      hooks = {
        SessionStart = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifarm];
          })
        ];

        SessionEnd = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifdarm];
          })
        ];

        BeforeAgent = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifrsig];
          })
        ];

        PreCompress = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifrsig];
          })
        ];

        Notification = [
          (kit.mkNotificationEntry {
            image = kit.images.gemini;
            title = "Gemini";
            message = "Awaiting your input";
            extraHooks = [
              (kit.mkAudioHook [kit.sounds.buzact])
            ];
          })
        ];

        BeforeTool = [
          {
            matcher = "run_shell_command";
            hooks = [
              {
                type = "command";
                command = "${config.home.homeDirectory}/${rtkPath}";
              }
            ];
          }
        ];
      };
    };
  };
}

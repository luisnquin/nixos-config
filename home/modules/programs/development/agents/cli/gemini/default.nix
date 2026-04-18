{
  config,
  agent,
  ...
}: let
  inherit (agent) mkAudioHook mkNotificationHook;
  inherit (agent.assets) sounds images;

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
    defaultModel = "gemini-2.5-flash";

    context = {
      GEMINI = agent.memories;
    };

    settings = {
      theme = "Default";
      vimMode = true;
      preferredEditor = "nano";
      autoAccept = false;
      secureModeEnabled = false;
      security.auth.selectedType = "oauth-personal";

      hooks = {
        SessionStart = [
          (mkAudioHook [sounds.ifarm])
        ];

        SessionEnd = [
          (mkAudioHook [sounds.ifdarm])
        ];
        BeforeAgent = [
          (mkAudioHook [sounds.ifrsig])
        ];

        PreCompress = [
          (mkAudioHook [sounds.ifrsig])
        ];
        Notification = [
          (mkNotificationHook images.gemini "Gemini" "Awaiting your input")
          (mkAudioHook [sounds.buzact])
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

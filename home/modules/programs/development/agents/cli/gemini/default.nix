{
  config,
  agent,
  pkgs,
  lib,
  ...
}: let
  inherit (agent.assets) sounds images;
in {
  programs.gemini-cli = {
    enable = true;
    defaultModel = "gemini-2.5-flash";

    context = {
      GEMINI = config.programs.claude-code.settings.memory.text;
    };

    settings = {
      theme = "Default";
      vimMode = true;
      preferredEditor = "nano";
      autoAccept = false;
      secureModeEnabled = false;
      security.auth.selectedType = "oauth-personal";

      hooks = let
        mkAudioHook = files: {
          matcher = "";
          hooks = [
            {
              type = "command";
              command = builtins.concatStringsSep " && " (
                map (mp3: "${pkgs.pulseaudio}/bin/paplay ${mp3}") files
              );
            }
          ];
        };

        mkNotificationHook = title: message: {
          matcher = "";
          hooks = [
            {
              type = "command";
              command = ''${lib.getExe pkgs.libnotify} -a "${title}" -i "${images.gemini}" "${title}" "${message}"'';
            }
          ];
        };
      in {
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
          (mkNotificationHook "Gemini" "Awaiting your input")
          (mkAudioHook [sounds.buzact])
        ];
      };
    };
  };
}

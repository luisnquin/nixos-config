{
  config,
  agent,
  ...
}: let
  inherit (agent) mkAudioHook mkNotificationHook;
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
      };
    };
  };
}

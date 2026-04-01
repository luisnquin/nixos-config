{
  pkgs,
  config,
  ...
}: {
  programs = {
    gemini-cli = {
      enable = true;
      defaultModel = "gemini-2.5-flash";
      settings = {
        theme = "Default";
        vimMode = true;
        preferredEditor = "nano";
        autoAccept = false;
        secureModeEnabled = false;
        security.auth.selectedType = "oauth-personal";
      };
    };

    claude-code = {
      enable = true;
      enableMcpIntegration = true;
      mcpServers = builtins.removeAttrs config.programs.mcp.servers [
        "expo"
        "supabase"
      ];

      memory.text = ''
        Do not use emojis in development outputs.
      '';

      includeCoAuthoredBy = false;

      permissions = {
        allow = [
          "Bash(git diff:*)"
          "Bash(git status:*)"
        ];

        ask = [
          "Bash(git push:*)"
        ];

        defaultMode = "acceptEdits";
        disableBypassPermissionsMode = "disable";
      };

      settings = {
        model = "claude-haiku-4-5-20251001";
        hooks = {
          SessionStart = [
            {
              matcher = "";
              hooks = [
                {
                  type = "command";
                  command = "${pkgs.pulseaudio}/bin/paplay ${./sounds/session-start.mp3}";
                }
              ];
            }
          ];
        };
      };
    };
  };
}

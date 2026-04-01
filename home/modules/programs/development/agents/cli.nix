{config, ...}: {
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
        Do not include the Co-authored-by trailer in commits.
        Do not use emojis in development outputs.
      '';
    };
  };
}

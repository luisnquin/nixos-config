{
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

    claude-code.enable = true;
  };
}

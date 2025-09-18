{
  programs.chromium = {
    enable = true;
    extraOpts = {
      "BrowserSignin" = 0;
      "SyncDisabled" = true;
      "PasswordManagerEnabled" = false;
      "SpellcheckEnabled" = true;
      "SpellcheckLanguage" = [
        "es-ES"
        "en-US"
      ];
      # "HomepageLocation" = "www.chromium.org";
      # "HomepageIsNewTabPage" = false;
    };
  };
}

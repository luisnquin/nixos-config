{libx, ...}: {
  programs.chromium = {
    enable = true;
    extraOpts = {
      "BackgroundModeEnabled" = false;
      "BrowserSignin" = 0;
      "SyncDisabled" = true;
      "LidCloseAction" = 1;

      "SpellcheckEnabled" = true;
      "SpellcheckLanguage" = [
        "es-ES"
        "en-US"
      ];

      "PasswordManagerEnabled" = false;
      "ThirdPartyPasswordManagersAllowed" = false;
      "PasswordLeakDetectionEnabled" = false;

      "DefaultSearchProviderEnabled" = true;
      "DefaultSearchProviderName" = "Google";
      "DefaultSearchProviderSearchURL" = "https://www.google.com/search?q={searchTerms}";

      "AutofillAddressEnabled" = false;
      "AutofillCreditCardEnabled" = false;

      "VirtualKeyboardEnabled" = false;
      "StickyKeysEnabled" = false;
      "DeviceAllowNewUsers" = false;

      "RestoreOnStartup" = 5;
      # "RestoreOnStartupURLs" = [
      #   "https://protect-ue.ismartlife.me/playback"
      # ];

      "BrowsingDataLifetime" = [
        {
          "data_types" = [
            "browsing_history"
          ];
          "time_to_live_in_hours" = 1;
        }
        {
          "data_types" = [
            "password_signin"
            "autofill"
          ];
          "time_to_live_in_hours" = 1;
        }
      ];

      "SafeBrowsingProtectionLevel" = 1;
      "DefaultPopupsSetting" = 2;

      "BlockThirdPartyCookies" = true;
      "DefaultCookiesSetting" = 4;
      "CookiesAllowedForUrls" = [
        (libx.base64.decode "aHR0cHM6Ly9wcm90ZWN0LXVlLmlzbWFydGxpZmUubWUK")
      ];

      "DnsOverHttpsMode" = "secure";
      "DnsOverHttpsTemplates" = "https://cloudflare-dns.com/dns-query";

      "ManagedBookmarks" = [
        {
          "toplevel_name" = "Absent friends";
        }
        {
          "name" = "ESC Configurator";
          "url" = "esc-configurator.com";
        }
        {
          "name" = "EdgeTX Buddy";
          "url" = "buddy.edgetx.org";
        }
        {
          "name" = "Betaflight Configurator";
          "url" = "app.betaflight.com";
        }
        # less waste of time
        {
          "name" = "Declaraciones SUNAT";
          "url" = "https://api-seguridad.sunat.gob.pe/v1/clientessol/03590141-c69c-438c-a36a-8ee2a3ad9747/oauth2/login";
        }
        {
          "name" = "Aduanas";
          "url" = "https://api-seguridad.sunat.gob.pe/v1/clientessol/59d39217-c025-4de5-b342-393b0f4630ab/oauth2/loginMenuSol";
        }
      ];
    };
  };
}

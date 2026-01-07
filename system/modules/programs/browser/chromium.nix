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
      "RestoreOnStartupURLs" = [
        "https://protect-ue.ismartlife.me/playback"
      ];

      "BrowsingDataLifetime" = [
        {
          "data_types" = [
            "browsing_history"
          ];
          "time_to_live_in_hours" = 24 / 2;
        }
        {
          "data_types" = [
            "password_signin"
            "autofill"
          ];
          "time_to_live_in_hours" = 2;
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
      ];
    };
  };
}

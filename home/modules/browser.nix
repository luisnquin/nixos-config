{
  inputs,
  system,
  pkgs,
  ...
}: {
  xdg.mimeApps = let
    associations = builtins.listToAttrs (map (name: {
        inherit name;
        value = let
          zen-browser = inputs.zen-browser.packages.${system}.twilight;
        in
          zen-browser.meta.desktopFile;
      }) [
        "application/x-extension-shtml"
        "application/x-extension-xhtml"
        "application/x-extension-html"
        "application/x-extension-xht"
        "application/x-extension-htm"
        "x-scheme-handler/unknown"
        "x-scheme-handler/mailto"
        "x-scheme-handler/chrome"
        "x-scheme-handler/about"
        "x-scheme-handler/https"
        "x-scheme-handler/http"
        "application/xhtml+xml"
        "application/json"
        "text/plain"
        "text/html"
      ]);
  in {
    associations.added = associations;
    defaultApplications = associations;
  };

  programs.zen-browser = {
    enable = true;
    policies = {
      AutofillAddressEnabled = true;
      AutofillCreditCardEnabled = false;
      DisableAppUpdate = true;
      DisableFeedbackCommands = true;
      DisableFirefoxStudies = true;
      DisablePocket = true; # save webs for later reading
      DisableTelemetry = true;
      DontCheckDefaultBrowser = true;
      NoDefaultBookmarks = true;
      OfferToSaveLogins = false;
    };
  };

  programs.librewolf = {
    enable = true;
    settings = {
      "beacon.enabled" = false;
      "browser.startup.page" = 3;
      "device.sensors.enabled" = false;
      "dom.battery.enabled" = false;
      "dom.event.clipboardevents.enabled" = false;
      "geo.enabled" = false;
      "media.peerconnection.enabled" = false;
      "privacy.clearHistory.cookiesAndStorage" = false;
      "privacy.clearHistory.siteSettings" = false;
      "privacy.firstparty.isolate" = true;
      "privacy.resistFingerprinting" = false;
      "privacy.trackingprotection.enabled" = true;
      "privacy.trackingprotection.socialtracking.enabled" = true;
      "webgl.disabled" = true; # may be annoying
    };
  };

  programs.chromium = {
    enable = true;
    package = pkgs.brave;
    extensions = let
      ids = [
        "ficfmibkjjnpogdcfhfokmihanoldbfe" # File Icons for GitHub and GitLab
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

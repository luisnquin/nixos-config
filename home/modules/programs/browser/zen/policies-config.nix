let
  mkLockedAttrs = builtins.mapAttrs (_: value: {
    Value = value;
    Status = "locked";
  });

  mkPluginUrl = id: "https://addons.mozilla.org/firefox/downloads/latest/${id}/latest.xpi";

  mkExtensionEntry = {
    id,
    pinned ? false,
  }: let
    base = {
      install_url = mkPluginUrl id;
      installation_mode = "force_installed";
    };
  in
    if pinned
    then base // {default_area = "navbar";}
    else base;

  mkExtensionSettings = builtins.mapAttrs (_: entry:
    if builtins.isAttrs entry
    then entry
    else mkExtensionEntry {id = entry;});
in {
  AutofillAddressEnabled = true;
  AutofillCreditCardEnabled = false;
  DisableAppUpdate = true;
  DisableFeedbackCommands = true;
  DisableFirefoxStudies = true;
  DisablePocket = true; # save webs for later reading
  DisableTelemetry = true;
  DontCheckDefaultBrowser = true;
  OfferToSaveLogins = false;
  EnableTrackingProtection = {
    Value = true;
    Locked = true;
    Cryptomining = true;
    Fingerprinting = true;
  };
  SanitizeOnShutdown = {
    FormData = true;
    Cache = true;
  };
  ExtensionSettings = mkExtensionSettings {
    "wappalyzer@crunchlabz.com" = mkExtensionEntry {
      id = "wappalyzer";
      pinned = true;
    };
    "uBlock0@raymondhill.net" = mkExtensionEntry {
      id = "ublock-origin";
      pinned = true;
    };
    # about:debugging#/runtime/this-firefox
    "{0814291e-c531-4741-a8e7-9a3e8f62bf71}" = "remove-youtube-tracking";
    "{3579f63b-d8ee-424f-bbb6-6d0ce3285e6a}" = "chameleon-ext";
    "{4590d8b8-3569-46e3-a571-cabfbaeab2c1}" = "no-youtube-shorts";
    "{74145f27-f039-47ce-a470-a662b129930a}" = "clearurls";
    "{762f9885-5a13-4abd-9c77-433dcd38b8fd}" = "return-youtube-dislikes";
    "{85860b32-02a8-431a-b2b1-40fbd64c9c69}" = "github-file-icons";
    "{861a3982-bb3b-49c6-bc17-4f50de104da1}" = "custom-user-agent-revived";
    "{a4c4eda4-fb84-4a84-b4a1-f7c1cbf2a1ad}" = "refined-github-";
    "{ef9e884b-b6d8-4544-b0de-82c46c5e95de}" = "sponsorblock";
    "{f52149fe-80cc-4d07-868d-c0e4a85453a0}" = "page-screenshot";
    "{fef652df-dd80-450e-b64a-567abeb3aa4b}" = "youtube-cards";
    "@searchengineadremover" = "searchengineadremover";
    "firefox-extension@steamdb.info" = "steam-database";
    "github-no-more@ihatereality.space" = "github-no-more";
    "github-repository-size@pranavmangal" = "gh-repo-size";
    "jid1-BoFifL9Vbdl2zQ@jetpack" = "decentraleyes";
    # "trackmenot@mrl.nyu.edu" = "trackmenot";
  };
  Preferences = mkLockedAttrs {
    "browser.aboutConfig.showWarning" = false;
    "browser.tabs.warnOnClose" = false;
    "media.videocontrols.picture-in-picture.video-toggle.enabled" = true;
    # Disable swipe gestures (Browser:BackOrBackDuplicate, Browser:ForwardOrForwardDuplicate)
    "browser.gesture.swipe.left" = "";
    "browser.gesture.swipe.right" = "";
    "browser.tabs.hoverPreview.enabled" = true;
    "browser.newtabpage.activity-stream.feeds.topsites" = false;
    "browser.topsites.contile.enabled" = false;
    "browser.translations.enable" = false;

    "privacy.resistFingerprinting" = true;
    "privacy.resistFingerprinting.randomization.canvas.use_siphash" = true;
    "privacy.resistFingerprinting.randomization.daily_reset.enabled" = true;
    "privacy.resistFingerprinting.randomization.daily_reset.private.enabled" = true;
    "privacy.resistFingerprinting.block_mozAddonManager" = true;
    "privacy.spoof_english" = 1;

    "privacy.firstparty.isolate" = true;
    "network.cookie.cookieBehavior" = 5;
    "dom.battery.enabled" = false;

    "gfx.webrender.all" = true;
    "network.http.http3.enabled" = true;
    "network.socket.ip_addr_any.disabled" = true; # disallow bind to 0.0.0.0
  };
}

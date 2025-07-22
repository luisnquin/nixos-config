{
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
}

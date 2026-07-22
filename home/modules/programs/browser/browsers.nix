{
  xdg.mimeApps = let
    associations = {
      "x-scheme-handler/http" = ["software.Browsers.desktop"];
      "x-scheme-handler/https" = ["software.Browsers.desktop"];
    };
  in {
    defaultApplications = associations;
    associations.added = associations;
  };

  programs.browsers = {
    enable = true;
    settings = {
      profile_order = ["zen-beta#default" "chromium#Default"];
      hidden_profiles = ["zen-beta#"]; # private

      rules =
        map (host: {
          url_pattern = host;
          opener.profile = "zen-beta#default";
        })
        [
          "app.betaflight.com"
          "esc-configurator.com"
          "buddy.edgetx.org"
        ];

      ui.theme = "Dark";
      behavior.unwrap_urls = true;
    };
  };
}

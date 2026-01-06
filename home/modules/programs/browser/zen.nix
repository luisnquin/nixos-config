{
  inputs,
  config,
  libx,
  pkgs,
  ...
}: {
  imports = [
    inputs.zen-browser.homeModules.beta
  ];

  xdg.mimeApps = let
    associations = builtins.listToAttrs (map (name: {
        inherit name;
        value = let
          zen-browser = config.programs.zen-browser.package;
        in
          zen-browser.meta.desktopFileName;
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

    policies = let
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
        "{a4c4eda4-fb84-4a84-b4a1-f7c1cbf2a1ad}" = "refined-github-";
        "{85860b32-02a8-431a-b2b1-40fbd64c9c69}" = "github-file-icons";
        "{762f9885-5a13-4abd-9c77-433dcd38b8fd}" = "return-youtube-dislikes";
        "{74145f27-f039-47ce-a470-a662b129930a}" = "clearurls";
        "github-no-more@ihatereality.space" = "github-no-more";
        "github-repository-size@pranavmangal" = "gh-repo-size";
        "firefox-extension@steamdb.info" = "steam-database";
        "@searchengineadremover" = "searchengineadremover";
        "jid1-BoFifL9Vbdl2zQ@jetpack" = "decentraleyes";
        "trackmenot@mrl.nyu.edu" = "trackmenot";
        "{861a3982-bb3b-49c6-bc17-4f50de104da1}" = "custom-user-agent-revived";
        "{3579f63b-d8ee-424f-bbb6-6d0ce3285e6a}" = "chameleon-ext";
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
    };

    profiles.default = rec {
      settings = {
        "zen.workspaces.continue-where-left-off" = true;
        "zen.workspaces.natural-scroll" = true;
        "zen.view.compact.hide-tabbar" = true;
        "zen.view.compact.hide-toolbar" = true;
        "zen.view.compact.animate-sidebar" = false;
        "zen.welcome-screen.seen" = true;
        "zen.urlbar.behavior" = "float";
      };

      bookmarks = {
        force = true;
        settings = [
          {
            name = "encore";
            tags = ["encore" "k9"];
            keyword = "encore";
            url = libx.base64.decode "aHR0cHM6Ly9hcHAuZW5jb3JlLmNsb3VkL2dhdGUtazktbXpuaQo=";
          }
          {
            name = "kernel.org";
            url = "https://www.kernel.org";
          }
          {
            name = "orders";
            url = "https://www.aliexpress.com/p/order/index.html";
            keyword = "orders";
          }
          {
            name = "s3";
            url = "https://sa-east-1.console.aws.amazon.com/s3/buckets?region=sa-east-1";
            keyword = "s3";
          }
          {
            name = "iam";
            url = "https://us-east-1.console.aws.amazon.com/iam/home?region=sa-east-1";
            keyword = "iam";
          }
        ];
      };

      pinsForce = true;
      pins = {
        "GitHub" = {
          id = "48e8a119-5a14-4826-9545-91c8e8dd3bf6";
          workspace = spaces."Rendezvous".id;
          url = "https://github.com";
          position = 101;
          isEssential = false;
        };
        "WhatsApp Web" = {
          id = "1eabb6a3-911b-4fa9-9eaf-232a3703db19";
          workspace = spaces."Rendezvous".id;
          url = "https://web.whatsapp.com/";
          position = 102;
          isEssential = false;
        };
        "Telegram Web" = {
          id = "5065293b-1c04-40ee-ba1d-99a231873864";
          url = "https://web.telegram.org/k/";
          position = 103;
          isEssential = true;
        };
        "PairDrop" = {
          id = "c70a0cd7-6ee8-470f-85c6-85a73a7a6196";
          url = "https://pairdrop.net/";
          position = 104;
          isEssential = true;
        };
      };

      containersForce = true;
      containers = {
        Shopping = {
          color = "yellow";
          icon = "dollar";
          id = 2;
        };
      };

      spacesForce = true;
      spaces = {
        "Rendezvous" = {
          id = "572910e1-4468-4832-a869-0b3a93e2f165";
          icon = "🎭";
          position = 1000;
          theme = {
            type = "gradient";
            colors = [
              {
                red = 216;
                green = 204;
                blue = 235;
                algorithm = "floating";
                type = "explicit-lightness";
              }
            ];
            opacity = 0.8;
            texture = 0.5;
          };
        };
        "Research" = {
          id = "ec287d7f-d910-4860-b400-513f269dee77";
          icon = "💌";
          position = 1001;
          theme = {
            type = "gradient";
            colors = [
              {
                red = 171;
                green = 219;
                blue = 227;
                algorithm = "floating";
                type = "explicit-lightness";
              }
            ];
            opacity = 0.2;
            texture = 0.5;
          };
        };
        "Shopping" = {
          id = "2441acc9-79b1-4afb-b582-ee88ce554ec0";
          icon = "💸";
          container = containers."Shopping".id;
          position = 1002;
        };
      };

      search = {
        force = true;
        default = "google";
        privateDefault = "ddg";
        engines = let
          nixSnowflakeIcon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
        in {
          "Nix Packages" = {
            urls = [
              {
                template = "https://search.nixos.org/packages";
                params = [
                  {
                    name = "type";
                    value = "packages";
                  }
                  {
                    name = "channel";
                    value = "unstable";
                  }
                  {
                    name = "query";
                    value = "{searchTerms}";
                  }
                ];
              }
            ];
            icon = nixSnowflakeIcon;
            definedAliases = ["pkgs"];
          };
          "Nix Options" = {
            urls = [
              {
                template = "https://search.nixos.org/options";
                params = [
                  {
                    name = "channel";
                    value = "unstable";
                  }
                  {
                    name = "query";
                    value = "{searchTerms}";
                  }
                ];
              }
            ];
            icon = nixSnowflakeIcon;
            definedAliases = ["nop"];
          };
          "Home Manager Options" = {
            urls = [
              {
                template = "https://home-manager-options.extranix.com/";
                params = [
                  {
                    name = "query";
                    value = "{searchTerms}";
                  }
                  {
                    name = "release";
                    value = "master"; # unstable
                  }
                ];
              }
            ];
            icon = nixSnowflakeIcon;
            definedAliases = ["hmop"];
          };

          "Google Maps" = {
            urls = [
              {
                template = "http://maps.google.com";
                params = [
                  {
                    name = "q";
                    value = "{searchTerms}";
                  }
                ];
              }
            ];
            definedAliases = ["maps" "gmaps"];
          };

          "ddg" = {
            urls = [
              {
                template = "https://duckduckgo.com";
                params = [
                  {
                    name = "q";
                    value = "{searchTerms}";
                  }
                  {
                    name = "origin";
                    value = "your_ass";
                  }
                ];
              }
            ];
            definedAliases = ["duck" "ddg" "dck" "dckk"];
          };

          MakerWorld = {
            urls = [
              {
                template = "https://makerworld.com/en/search/models";
                params = [
                  {
                    name = "keyword";
                    value = "{searchTerms}";
                  }
                ];
              }
            ];
            definedAliases = ["maker" "mw"];
          };

          Printables = {
            urls = [
              {
                template = "https://www.printables.com/search/models";
                params = [
                  {
                    name = "q";
                    value = "{searchTerms}";
                  }
                ];
              }
            ];
            definedAliases = ["pt" "print" "printables"];
          };

          bing.metaData.hidden = "true";
        };
      };
    };
  };
}

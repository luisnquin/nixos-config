{
  inputs,
  libx,
  pkgs,
  ...
}: {
  imports = [
    inputs.zen-browser.homeModules.beta
    ./xdg.nix
  ];

  programs.zen-browser = {
    enable = true;

    policies = import ./policies-config.nix;

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

      # mods = [
      #   "c01d3e22-1cee-45c1-a25e-53c0f180eea8" # Ghost Tabs
      # ];

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

      search = import ./search-config.nix {inherit pkgs;};

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

      keyboardShortcutsVersion = 13;
      keyboardShortcuts = [
        {
          id = "zen-compact-mode-toggle";
          key = "c";
          modifiers.control = true;
          modifiers.alt = true;
        }
        {
          id = "key_savePage";
          key = "s";
          modifiers.control = true;
        }
        {
          id = "key_quitApplication";
          disabled = true;
        }
      ];
    };
  };
}

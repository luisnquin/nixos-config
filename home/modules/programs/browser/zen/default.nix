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
    languagePacks = ["en-US"];
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

      mods = [
        "253a3a74-0cc4-47b7-8b82-996a64f030d5" # Floating History
        "4ab93b88-151c-451b-a1b7-a1e0e28fa7f8" # No Sidebar Scrollbar
        "7190e4e9-bead-4b40-8f57-95d852ddc941" # Tab title fixes
        "803c7895-b39b-458e-84f8-a521f4d7a064" # Hide Inactive Workspaces
        "906c6915-5677-48ff-9bfc-096a02a72379" # Floating Status Bar
        "a6335949-4465-4b71-926c-4a52d34bc9c0" # Better Find Bar
        "c6813222-6571-4ba6-8faf-58f3343324f6" # Disable Rounded Corners
        "c8d9e6e6-e702-4e15-8972-3596e57cf398" # Zen Back Forward
        "cb15abdb-0514-4e09-8ce5-722cf1f4a20f" # Hide Extension Name
        "d8b79d4a-6cba-4495-9ff6-d6d30b0e94fe" # Better Active Tab
        "e122b5d9-d385-4bf8-9971-e137809097d0" # No Top Sites
        "f7c71d9a-bce2-420f-ae44-a64bd92975ab" # Better Unloaded Tabs
        "fd24f832-a2e6-4ce9-8b19-7aa888eb7f8e" # Quietify
      ];

      bookmarks = {
        force = true;
        settings = let
          gmailEntries = map (i: rec {
            keyword = "l${toString i}";
            name = keyword;
            url = "https://mail.google.com/mail/u/${toString i}/#inbox";
          }) [0 1 2 3];
        in
          [
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
            {
              name = "firebase";
              url = "https://console.firebase.google.com/u/0/";
              keyword = "fire";
            }
            {
              name = "novu";
              url = "https://dashboard-v2.novu.co/env/dev_env_gQwrIEDTzbHL4coa";
              keyword = "novu";
            }
            {
              name = "supabase";
              url = "https://supabase.com/dashboard/project/mjkvxcziwkwohxpuejak";
              keyword = "supa";
            }
            {
              name = "krear3d";
              url = "https://www.tiendakrear3d.com/productos/filamentos/";
              keyword = "3d";
            }
            {
              name = "digitalz3d";
              url = "http://www.digitalz3d.com/filamentos";
              keyword = "3dd";
            }
          ]
          ++ gmailEntries;
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
                red = 123;
                green = 56;
                blue = 58;
                algorithm = "analogous";
                type = "explicit-lightness";
                lightness = 35;
                position.x = 301;
                position.y = 176;
                primary = true;
                custom = false;
              }
              {
                red = 123;
                green = 110;
                blue = 55;
                algorithm = "analogous";
                type = "explicit-lightness";
                lightness = 35;
                position.x = 260;
                position.y = 271;
                primary = false;
                custom = false;
              }
              {
                red = 122;
                green = 56;
                blue = 114;
                algorithm = "analogous";
                type = "explicit-lightness";
                lightness = 35;
                position.x = 255;
                position.y = 84;
                primary = false;
                custom = false;
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
          id = "zen-toggle-sidebar";
          key = "x";
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

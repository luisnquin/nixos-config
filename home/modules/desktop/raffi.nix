{
  programs.raffi = {
    enable = true;

    settings = {
      version = 1;

      general = {
        ui_type = "fuzzel";
        default_script_shell = "bash";
        no_icons = true;
        max_history = 10;
        sort_mode = "hybrid";

        fallbacks = [
          ''addons.web_searches."Google"''
          ''addons.script_filters."Search Google"''
        ];
      };

      launchers = {
        zen = {
          binary = "zen-beta";
          description = "Web browser";
        };

        terminal = {
          binary = "ghostty";
          description = "Open a new terminal";
        };
      };

      addons = {
        web_searches = [
          {
            name = "Google";
            keyword = "g";
            url = "https://google.com/search?q={query}";
          }
          {
            name = "DuckDuckGo";
            keyword = "ddg";
            url = "https://duckduckgo.com/?q={query}";
          }
        ];

        calculator.enabled = true;

        currency = {
          enabled = true;
          currencies = ["USD" "EUR" "PEN"];
          default_currency = "USD";
          trigger = "curr";
        };

        file_browser = {
          enabled = true;
          show_hidden = true;
        };
      };
    };
  };
}

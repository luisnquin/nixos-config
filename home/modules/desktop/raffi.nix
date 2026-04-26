{
  programs.raffi = {
    enable = true;

    settings = {
      version = 1;

      general = {
        ui_type = "native";
        default_script_shell = "bash";
        no_icons = true;
        max_history = 10;
        theme = "dark";
        font_size = 15.0;
        font_family = "Inconsolata";
        window_width = 420.0;
        window_height = 320.0;
        padding = 5.0;
        sort_mode = "hybrid";

        fallbacks = [
          ''addons.web_searches."Google"''
          ''addons.script_filters."Search Google"''
        ];

        theme_colors = {
          bg_base = "#1e1e2e";
          bg_input = "#313244";
          accent = "#cba6f7";
          accent_hover = "#89b4fa";
          text_main = "#cdd6f4";
          text_muted = "#6c7086";
          selection_bg = "#45475a";
          border = "#585b70";
        };
      };

      launchers = {
        zen = {
          binary = "zen-beta";
          description = "Web browser";
        };

        terminal = {
          binary = "ghostty";
          icon = "terminal";
          description = "Open a new terminal";
        };
      };

      addons = {
        web_searches = [
          {
            name = "Google";
            keyword = "g";
            url = "https://google.com/search?q={query}";
            icon = "google";
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

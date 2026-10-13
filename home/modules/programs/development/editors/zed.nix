{pkgs, ...}: let
  biomeFormat = {
    formatter.language_server.name = "biome";
  };
in {
  programs.zed-editor = {
    enable = true;

    mutableUserSettings = false;
    mutableUserKeymaps = false;
    mutableUserTasks = false;
    mutableUserDebug = false;

    extensions = [
      "assembly"
      "astro"
      "biome"
      "csv"
      "dart"
      "docker-compose"
      "dockerfile"
      "env"
      "gdscript"
      "git-firefly"
      "glsl"
      "graphql"
      "html"
      "hyprlang"
      "ini"
      "just"
      "kdl"
      "log"
      "lua"
      "make"
      "material-icon-theme"
      "mdx"
      "neocmake"
      "nginx"
      "nix"
      "php"
      "prisma"
      "proto"
      "sql"
      "ssh-config"
      "swift"
      "terraform"
      "tokyo-night"
      "toml"
      "typos"
      "typst"
      "vue"
      "xml"
      "zig"
    ];

    extraPackages = with pkgs; [
      alejandra
      biome
      clang-tools
      gopls
      lua-language-server
      neocmakelsp
      nil
      nodejs
      package-version-server
      rust-analyzer
      shellcheck
      shfmt
      taplo
      terraform-ls
      tinymist
      typos-lsp
    ];

    userSettings = {
      disable_ai = true;

      telemetry = {
        diagnostics = false;
        metrics = false;
      };
      auto_update = false;

      theme = {
        mode = "dark";
        light = "One Light";
        dark = "Tokyo Night";
      };
      icon_theme = "Material Icon Theme";
      title_bar.show_sign_in = false;
      ui_font_size = 16;
      buffer_font_size = 16;
      buffer_font_family = "Cascadia Code";

      restore_on_startup = "last_session";
      autosave.after_delay.milliseconds = 1000;

      format_on_save = "on";
      prettier.allowed = false;

      inlay_hints.enabled = true;
      diagnostics.inline.enabled = true;

      file_types = {
        JSON = ["flake.lock"];
        JSONC = ["encore.app"];
        YAML = [".senv"];
      };

      languages = {
        Nix = {
          language_servers = ["nil" "!nixd"];
          formatter.language_server.name = "nil";
        };
        JavaScript = biomeFormat;
        TypeScript = biomeFormat;
        TSX = biomeFormat;
        JSON = biomeFormat;
        JSONC = biomeFormat;
        CSS = biomeFormat;
      };

      lsp = {
        nil.settings.formatting.command = ["alejandra"];
        biome.settings.require_config_file = true;
        rust-analyzer.initialization_options.check.command = "clippy";
      };
    };

    userKeymaps = [
      {
        context = "Workspace";
        bindings = {
          ctrl-space = "terminal_panel::Toggle";
          ctrl-alt-left = "pane::GoBack";
          ctrl-alt-right = "pane::GoForward";
        };
      }
      {
        context = "Editor";
        bindings.ctrl-space = "terminal_panel::Toggle";
      }
      {
        context = "Terminal";
        bindings.ctrl-space = "terminal_panel::Toggle";
      }
    ];
  };
}

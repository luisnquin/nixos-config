{
  pkgs,
  lib,
  ...
}: let
  # Stopgap: own plugins.json declaratively until the herdr module exposes a
  # native `plugins` option. importTOML reads each manifest at eval time (IFD).
  plugins = [
    pkgs.herdr-autoname
    pkgs.herdr-pluck
    pkgs.herdr-sesh
  ];

  pluginEntry = pkg: let
    manifest = lib.importTOML "${pkg}/herdr-plugin.toml";
  in {
    plugin_id = manifest.id;
    name = manifest.name;
    version = manifest.version;
    min_herdr_version = manifest.min_herdr_version or "";
    manifest_path = "${pkg}/herdr-plugin.toml";
    plugin_root = "${pkg}";
    enabled = true;
    source.kind = "local";
  };

  pluginRegistry = pkgs.writeText "herdr-plugins.json" (
    builtins.toJSON (map pluginEntry plugins)
  );
in {
  programs.herdr = {
    enable = true;

    settings = {
      onboarding = false;

      theme.name = "terminal";
      terminal.new_cwd = "follow";
      worktrees.directory = "~/.herdr/worktrees";
      session.resume_agents_on_restore = true;

      ui = {
        toast = {
          delivery = "system";
          delay_seconds = 1;
        };

        sidebar.agents.rows_by_agent = builtins.listToAttrs (
          map (agent: {
            name = agent;
            value = [
              ["state_icon" "agent" "state_text"]
              ["terminal_title_stripped"]
              ["workspace" "tab"]
            ];
          }) [
            "claude"
            "codex"
            "opencode"
            "pi"
          ]
        );
      };

      keys = {
        split_vertical = ["prefix+v" "prefix+percent" "prefix+|" "prefix+backtick"];
        split_horizontal = ["prefix+s" "prefix+double_quote" "prefix+minus" "prefix+slash"];
        settings = "prefix+shift+s";
        rename_tab = "prefix+comma";

        next_workspace = "prefix+tab";
        previous_workspace = "prefix+shift+tab";

        command = [
          {
            key = "prefix+alt+g";
            type = "pane";
            command = "lazygit";
            description = "lazygit";
          }
        ];
      };
    };
  };

  # Real file, not a symlink: herdr rewrites plugins.json under a lock.
  home.activation.herdrPluginRegistry = lib.hm.dag.entryAfter ["writeBoundary"] ''
    run install -Dm644 ${pluginRegistry} "$HOME/.config/herdr/plugins.json"
  '';

  programs.zsh.initContent = lib.mkAfter ''
    source ${pkgs.herdr-autoname}/shell/hook.zsh
  '';
}

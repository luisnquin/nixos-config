{...}: {
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
}

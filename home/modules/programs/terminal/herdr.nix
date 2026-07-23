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

      # Mirror Ghostty's bindings. Move settings away from its default prefix+s.
      keys = {
        split_horizontal = "prefix+s";
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

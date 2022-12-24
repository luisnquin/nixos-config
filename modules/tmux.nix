{
  config,
  pkgs,
  lib,
  ...
}: {
  programs.tmux = {
    enable = true;
    clock24 = true;
    terminal = "xterm-256color";
    newSession = false;
    historyLimit = 1000000;

    plugins = with pkgs; [
      tmuxPlugins.online-status
      tmuxPlugins.onedark-theme
      tmuxPlugins.fzf-tmux-url
      # tmuxPlugins.continuum
      tmuxPlugins.tmux-fzf
      tmuxPlugins.sidebar
      tmuxPlugins.sysstat
    ];

    extraConfig = ''
      setw -g mouse on
      set -g window-status-separator ""

      set-option -ga terminal-overrides ",*256col*:Tc:RGB"

      # set -g automatic-rename off
      # set -g status-left-length 40
      # set -g status-left "#S #[fg=white]#[fg=yellow]#I #[fg=cyan]#P"
      # set -g status-bg black
      # set -g status-fg magenta

      # TODO: add client env
      # set -g status-right "#{sysstat_cpu} | #{sysstat_mem} | #{sysstat_swap} | #{sysstat_loadavg} | #[fg=cyan]#(echo $USER)#[default]@#H"

      set -g status-right '#(${pkgs.gitmux}/bin/gitmux "#{pane_current_path}")'
    '';
  };

  environment.etc."gitmux.conf" = with lib; {
    # Until we find a way to correctly handle YAML files
    text = ''
      tmux:
        symbols:
          branch: '⎇ '
          hashprefix: ':'
          ahead: ↑·
          behind: ↓·
          staged: '● '
          conflict: '✖ '
          modified: '✚ '
          untracked: '… '
          stashed: '⚑ '
          clean: ✔
        styles:
          clear: '#[fg=default]'
          state: '#[fg=red,bold]'
          branch: '#[fg=white,bold]'
          remote: '#[fg=cyan]'
          staged: '#[fg=green,bold]'
          conflict: '#[fg=red,bold]'
          modified: '#[fg=red,bold]'
          untracked: '#[fg=magenta,bold]'
          stashed: '#[fg=cyan,bold]'
          clean: '#[fg=green,bold]'
          divergence: '#[fg=default]'
        layout: [branch, .., remote-branch, divergence, ' - ', flags]
        options:
          branch_max_len: 0
          branch_trim: right
    '';

    /*
    text = generators.toYAML {} {
      tmux = {
        symbols = {
          branch = "⎇ ";
          hashprefix = ":";
          ahead = "↑·";
          behind = "↓·";
          staged = "● ";
          conflict = "✖ ";
          modified = "✚ ";
          untracked = "… ";
          stashed = "⚑ ";
          clean = "✔";
        };

        styles = {
          clear = "#[fg=default]";
          state = "#[fg=red,bold]";
          branch = "#[fg=white,bold]";
          remote = "#[fg=cyan]";
          staged = "#[fg=green,bold]";
          conflict = "#[fg=red,bold]";
          modified = "#[fg=red,bold]";
          untracked = "#[fg=magenta,bold]";
          stashed = "#[fg=cyan,bold]";
          clean = "#[fg=green,bold]";
          divergence = "#[fg=default]";
        };

        layout = ["branch" "..." "remote-branch" "divergence" " - " "flags"];

        options = {
          branch_max_len = 0;
          branch_trim = "right";
        };
      };
    };
    */
  };
}

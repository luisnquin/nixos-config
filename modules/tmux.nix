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

    plugins = with pkgs.tmuxPlugins; [
      online-status
      onedark-theme
      fzf-tmux-url
      tmux-fzf
      sidebar
      sysstat
    ];

    extraConfig = ''
      setw -g mouse on
      bind-key -T copy-mode-vi MouseDragEnd1Pane send-keys -X copy-pipe-and-cancel "xclip"

      set -g window-status-separator ""
      set-option -ga terminal-overrides ",*256col*:Tc:RGB"

      set -g status-right '#(${pkgs.gitmux}/bin/gitmux "#{pane_current_path}")'
    '';
  };

  environment = {
    systemPackages = [pkgs.tmux];

    etc."gitmux.conf".text = builtins.readFile ../dots/etc/gitmux.conf;
  };
}
# set -g automatic-rename off
# set -g status-left-length 40
# set -g status-left "#S #[fg=white]#[fg=yellow]#I #[fg=cyan]#P"
# set -g status-bg black
# set -g status-fg magenta
# set -g status-right "#{sysstat_cpu} | #{sysstat_mem} | #{sysstat_swap} | #{sysstat_loadavg} | #[fg=cyan]#(echo $USER)#[default]@#H"


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
      bind-key -T copy-mode-vi MouseDragEnd1Pane send-keys -X copy-pipe-and-cancel "xclip" # It's not really working, lol

      set -g window-status-separator ""
      set-option -ga terminal-overrides ",*256col*:Tc:RGB"

      # Refresh 'status-left' and 'status-right' more often, from every 15s to 5s
      set -g status-interval 5

      # Increase tmux messages display duration from 750ms to 4s
      set -g display-time 4000

      set -g status-right '#(${pkgs.gitmux}/bin/gitmux "#{pane_current_path}")'
    '';
  };

  environment = {
    systemPackages = [pkgs.tmux];

    etc."gitmux.conf".text = builtins.readFile ../../dots/etc/gitmux.conf;
  };
}

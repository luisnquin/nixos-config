{
  config,
  pkgs,
  lib,
  ...
}: let
  tmux-plugins = with pkgs; [
    tmuxPlugins.online-status
    tmuxPlugins.onedark-theme
    tmuxPlugins.fzf-tmux-url
    # tmuxPlugins.continuum
    tmuxPlugins.sidebar
    tmuxPlugins.sysstat
  ];

  tmux-plugins-dependencies = with pkgs; [
    fzf
  ];
in {
  environment.systemPackages =
    [
      pkgs.tmux
    ]
    ++ tmux-plugins-dependencies
    ++ tmux-plugins;

  programs.tmux = {
    enable = true;
    clock24 = true;
    terminal = "xterm-256color";
    newSession = false;
    historyLimit = 1000000;

    extraConfig = ''
      setw -g mouse on
      set -g window-status-separator ""

      set-option -ga terminal-overrides ",*256col*:Tc:RGB"

      # set -g automatic-rename off

      set -g status-bg black
      set -g status-fg magenta

      # set -g status-left-length 40
      # set -g status-left "#S #[fg=white]#[fg=yellow]#I #[fg=cyan]#P"

      set -g status-right "#{sysstat_cpu} | #{sysstat_mem} | #{sysstat_swap} | #{sysstat_loadavg} | #[fg=cyan]#(echo $USER)#[default]@#H"

      ${lib.concatStrings (map (x: "run-shell ${x.rtp}\n") tmux-plugins)}
    '';
  };
}

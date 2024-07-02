{pkgs, ...}: {
  programs.tmux = {
    enable = true;
    package = pkgs.tmux;
    extraConfig = builtins.readFile ./dots/.tmux.conf;
  };

  home = {
    packages = [
      pkgs.gitmux
    ];

    sessionPath = ["$HOME/.tmux/plugins/tpm"];
  };

  xdg.configFile = {
    ".gitmux.conf".text = builtins.readFile ./dots/.gitmux.conf;
  };
}

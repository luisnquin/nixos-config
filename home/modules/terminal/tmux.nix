{pkgs, ...}: {
  programs.tmux = {
    enable = true;
    package = pkgs.tmux;
    extraConfig = builtins.readFile ./dots/.tmux.conf;
  };

  home.packages = [
    pkgs.gitmux
  ];

  xdg.configFile = {
    ".gitmux.conf".text = builtins.readFile ./dots/.gitmux.conf;
  };
}

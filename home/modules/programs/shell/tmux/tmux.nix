{pkgs, ...}: {
  programs.tmux = {
    enable = true;
    package = pkgs.tmux;
    extraConfig = builtins.readFile ./tmux.conf;
  };

  home.packages = [
    pkgs.gitmux
  ];

  xdg.configFile = {
    ".gitmux.conf".text = builtins.readFile ./gitmux.conf;
  };
}

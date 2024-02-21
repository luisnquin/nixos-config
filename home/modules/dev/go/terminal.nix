{config, ...}: {
  home.sessionPath = ["$GORROT" "$GOPATH/bin"];

  programs.zsh = {
    shellAliases = {
      gest = "go clean -testcache && richgo test -v";
    };

    initExtra = builtins.readFile (builtins.path {
      name = "go-shrc";
      path = ./dots/shell.sh;
    });
  };
}

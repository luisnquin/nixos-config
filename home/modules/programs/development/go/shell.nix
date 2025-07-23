{pkgs, ...}: {
  home.sessionPath = ["$GORROT" "$GOPATH/bin"];

  programs.zsh = {
    shellAliases = {
      gest = "go clean -testcache && ${pkgs.richgo}/bin/richgo test -v";
    };

    initContent = builtins.readFile (builtins.path {
      name = "go-shrc";
      path = ./shell.sh;
    });
  };
}

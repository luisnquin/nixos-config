{pkgs, ...}: {
  home = {
    packages = with pkgs; [
      air
      delve
      errcheck
      gcc
      go_1_21
      go-protobuf
      gofumpt
      golangci-lint
      gopls
      gotools
      govulncheck
      grpc-tools
      richgo
      tinygo
      unconvert
    ];

    sessionVariables = {
      PATH = "$PATH:$GORROT:$GOPATH/bin";
      GOPATH = "/home/$USER/go";
    };
  };

  programs.zsh.shellAliases = {
    gotry = "xdg-open https://go.dev/play >>/dev/null";
    gest = "go clean -testcache && richgo test -v";
  };
}

{
  pkgs,
  pkgsx,
  ...
}: {
  home = {
    packages = with pkgs; [
      air # hot reload
      delve # debugger
      errcheck # to check not handled errors
      gcc # to link C code
      go_1_21 # compiler
      go-protobuf
      gofumpt # gofmt but better
      golangci-lint # linter
      gopls # Go language server
      gotools # useful stuff for go
      govulncheck # audit dependencies
      grpc-tools
      panicparse # to help to debug panic errors
      richgo # go test but with better outputs
      tinygo
      unconvert # linter to check unnecessary type conversions
    ];

    sessionVariables = {
      PATH = "$PATH:$GORROT:$GOPATH/bin";
      GOPATH = "/home/$USER/go";
    };
  };

  programs.zsh.shellAliases = {
    gest = "go clean -testcache && richgo test -v";
  };
}

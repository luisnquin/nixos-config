{pkgs, ...}: {
  home = {
    packages = with pkgs; [
      air # hot reload
      delve # debugger
      errcheck # to check not handled errors
      gcc # to link C code
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
    };
  };

  xdg.configFile = {
    "go/env".text = builtins.readFile ../../dots/go/env;
  };

  programs = {
    go = {
      enable = true;
      package = pkgs.go_1_21;
      # I don't get the point of these packages because it adds
      # them to mod $GOPATH/src instead of $GOPATH/pkg/mod
      #
      # packages = {
      #   "github.com/goccy/go-json" = builtins.fetchGit {
      #     url = "https://github.com/goccy/go-json";
      #     name = "go-package";
      #     rev = "df897aec9dc4228e585e8127b4db026d506d2b3c";
      #   };
      #   "github.com/samber/lo" = builtins.fetchGit {
      #     url = "https://github.com/samber/lo";
      #     name = "go-package";
      #     rev = "df897aec9dc4228e585e8127b4db026d506d2b3c";
      #   };
      # };

      goBin = "go/bin";
      goPath = "go";
    };

    zsh.shellAliases = {
      gest = "go clean -testcache && richgo test -v";
    };
  };
}

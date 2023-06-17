{
  config,
  pkgs,
  ...
}: {
  environment = {
    systemPackages = with pkgs; [
      air
      delve
      errcheck
      gcc
      go_1_20
      go-protobuf
      gofumpt
      golangci-lint
      gopls
      gotools
      govulncheck
      grpc-tools
      richgo
      tinygo
    ];

    shellAliases = {
      gotry = "xdg-open https://go.dev/play >>/dev/null";
      gest = "go clean -testcache && richgo test -v";
    };

    variables = {
      PATH = "$PATH:$GORROT:$GOPATH/bin";
      GOPATH = "/home/$USER/go";
    };
  };
}

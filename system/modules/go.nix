{
  config,
  pkgs,
  ...
}: {
  environment = {
    systemPackages = with pkgs; [
      air
      delve
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
      gmt = "go mod tidy";
    };

    variables = {
      PATH = "$PATH:$GORROT:$GOPATH/bin";
      GOPATH = "/home/$USER/go";
    };
  };
}

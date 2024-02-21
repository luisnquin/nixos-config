{pkgs, ...}: {
  home.packages = with pkgs; [
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
    tinygo
    unconvert # linter to check unnecessary type conversions
  ];
}

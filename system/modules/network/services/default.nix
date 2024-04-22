self: {
  imports = [
    (import ./static-web-server.nix self)
    (import ./rustypaste.nix self)
  ];
}

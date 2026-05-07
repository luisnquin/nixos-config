{pkgs}: let
  inherit (pkgs) lib;
in
  builtins.mapAttrs (_name: path:
    import path {
      inherit pkgs lib;
    }) {
    formats = ./formats.nix;
    base64 = ./base64.nix;
    fs = ./fs.nix;
    comms = ./comms.nix;
  }
  // import ./metadata.nix {inherit lib;}

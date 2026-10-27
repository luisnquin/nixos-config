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
    notify = ./notify.nix;
    nvidia = ./nvidia.nix;
  }
  // import ./metadata.nix {inherit lib;}

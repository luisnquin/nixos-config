{
  nixosConfig,
  config,
  pkgs,
  libx,
  lib,
  ...
}: {
  _module.args = import ./lib.nix {inherit nixosConfig pkgs lib libx config;};

  imports = [
    ./apps
    ./cli
    ./hooks
    ./mcp.nix
    ./options
    ./skills.nix
  ];
}

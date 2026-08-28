{
  nixosConfig,
  inputs,
  config,
  pkgs,
  libx,
  lib,
  ...
}: {
  _module.args = import ./lib.nix {inherit nixosConfig inputs pkgs lib libx config;};

  imports = [
    ./apps
    ./cli
    ./hooks
    ./mcp.nix
    ./options
    ./skills.nix
  ];
}

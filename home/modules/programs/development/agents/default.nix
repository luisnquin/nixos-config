{
  config,
  pkgs,
  lib,
  ...
}: {
  _module.args = import ./lib.nix {inherit pkgs lib config;};

  imports = [
    ./apps
    ./cli
    ./hooks
    ./mcp.nix
    ./options
    ./skills.nix
  ];
}

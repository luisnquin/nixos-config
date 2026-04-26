{
  config,
  pkgs,
  lib,
  ...
}: {
  _module.args = import ./lib.nix {inherit pkgs lib config;};

  imports = [
    ./apps
    ./options
    ./cli/codex
    ./cli/gemini
    ./cli/claude
    ./cli/opencode
    ./hooks
    ./mcp.nix
    ./skills.nix
  ];
}

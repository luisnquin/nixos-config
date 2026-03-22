{
  config,
  pkgs,
  ...
}: {
  home.file.".cursor/mcp.json".text = builtins.toJSON config.programs.mcp.vendorServers;

  programs.cursor = {
    enable = true;
    package = pkgs.code-cursor;
    mutableExtensionsDir = true;
    inherit (config.programs.antigravity) profiles;
  };
}

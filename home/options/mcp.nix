{
  pkgs,
  lib,
  ...
}:
with lib; let
  jsonFormat = pkgs.formats.json {};
in {
  options.programs.mcp = {
    vendorServers = mkOption {
      inherit (jsonFormat) type;
      default = {};
      description = ''
        Vendor-specific MCP server definitions (e.g. Cursor, Antigravity).
      '';
    };
  };
}

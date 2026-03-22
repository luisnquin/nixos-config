{
  config,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.mcp;

  normalizeServer = name: server: let
    hasUrl = server ? url;
    hasServerUrl = server ? serverUrl;
  in
    nameValuePair name (
      server
      // optionalAttrs (hasUrl && !hasServerUrl) {
        serverUrl = server.url;
      }
    );

  normalizedServers =
    mapAttrs' normalizeServer cfg.servers;
in {
  options.programs.mcp = {
    vendorServers = mkOption {
      type = types.attrs;
      readOnly = true;
      description = "Normalized MCP vendor servers (auto-generated).";
    };
  };

  config = mkIf cfg.enable {
    programs.mcp.vendorServers = {
      mcpServers = normalizedServers;
    };
  };
}

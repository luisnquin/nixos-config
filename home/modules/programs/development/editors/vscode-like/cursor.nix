{
  mkAgentKit,
  config,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};
in {
  home.file.".cursor/mcp.json".text = builtins.toJSON (kit.mkMcpServers {
    normalizeServerUrl = true;
  });

  home.activation.copyCursorSettings = lib.hm.dag.entryAfter ["writeBoundary"] ''
    mkdir -p ~/.config/Cursor/User
    rm -f ~/.config/Cursor/User/settings.json
    cp ${./settings.json} ~/.config/Cursor/User/settings.json
    chmod +w ~/.config/Cursor/User/settings.json
  '';

  programs.cursor = {
    enable = true;
    package = pkgs.code-cursor;
    # package = pkgs.llm-agents.code-cursor;
    mutableExtensionsDir = true;
    inherit (config.programs.antigravity) profiles;
  };
}

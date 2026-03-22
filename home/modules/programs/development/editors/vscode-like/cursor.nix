{
  config,
  pkgs,
  lib,
  ...
}: {
  home.file.".cursor/mcp.json".text = builtins.toJSON config.programs.mcp.vendorServers;

  home.activation.copyCursorSettings = lib.hm.dag.entryAfter ["writeBoundary"] ''
    mkdir -p ~/.config/Cursor/User
    rm -f ~/.config/Cursor/User/settings.json
    cp ${./settings.json} ~/.config/Cursor/User/settings.json
    chmod +w ~/.config/Cursor/User/settings.json
  '';

  programs.cursor = {
    enable = true;
    package = pkgs.code-cursor;
    mutableExtensionsDir = true;
    inherit (config.programs.antigravity) profiles;
  };
}

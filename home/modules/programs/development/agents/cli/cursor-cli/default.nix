{
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
in {
  programs.cursor-agent = {
    enable = true;
    package = pkgs.symlinkJoin {
      name = "cursor-agent";
      paths = [pkgs.cursor-cli];
      buildInputs = [pkgs.makeWrapper];

      postBuild = ''
        wrapProgram "$out/bin/cursor-agent" \
          --set PINENTRY_USER_DATA gui

        ln -s cursor-agent "$out/bin/agent"
      '';
    };
    enableMcpIntegration = true;
    rules = {
      "global.mdc" = ''
        ---
        alwaysApply: true
        ---
        ${kit.memories}
      '';
    };
    settings = {
      attribution = {
        attributeCommitsToAgent = false;
        attributePRsToAgent = false;
      };
    };
  };
}

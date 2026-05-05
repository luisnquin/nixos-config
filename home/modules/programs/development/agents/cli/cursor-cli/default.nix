{
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};
in {
  programs.cursor-agent = {
    enable = true;
    package = pkgs.cursor-cli;
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

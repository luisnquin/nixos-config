{pkgs, ...}: {
  programs.cursor-agent = {
    enable = true;
    package = pkgs.cursor-cli;
    enableMcpIntegration = true;
    settings = {
      attribution = {
        attributeCommitsToAgent = false;
        attributePRsToAgent = false;
      };
    };
  };
}

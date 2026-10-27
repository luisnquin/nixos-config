{
  mkAgentKit,
  config,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};

  # pi has no allow/deny tool-permission model; its only gate is per-project
  # trust. Mirror the kit's autonomous switch used by the other agents.
  isAutonomous = (config.agents.permissions.profile or "standard") == "autonomous";
in {
  home.file.".pi/agent/mcp.json".text = builtins.toJSON {
    mcpServers = lib.mapAttrs (_: server:
      builtins.intersectAttrs {
        url = null;
        command = null;
        args = null;
        env = null;
        cwd = null;
      } server
      // {exposure = "deferred";}) (kit.mkMcpServers {});
  };

  programs.pi-coding-agent = {
    enable = true;
    package = pkgs.llm-agents.pi;

    # https://pi.dev/docs/latest/settings
    settings = {
      defaultProvider = "openai";
      defaultModel = "gpt-6-sol";
      defaultThinkingLevel = "medium";

      theme = "dark";
      externalEditor = "nano";
      quietStartup = true;
      terminal.showTerminalProgress = true;

      packages = [
        "npm:pi-subagents@0.83.0"
        "npm:pi-diff-review@0.1.29"
      ];

      enableInstallTelemetry = false;
      enableAnalytics = false;

      defaultProjectTrust =
        if isAutonomous
        then "trusted"
        else "ask";

      enabledModels = [
        "openai/gpt-6*"
      ];

      compaction = {
        enabled = true;
        reserveTokens = 16384;
        keepRecentTokens = 20000;
      };

      retry = {
        enabled = true;
        maxRetries = 3;
      };

      # https://pi.dev/docs/latest/extensions
      # rtk rewrites bash commands to token-optimized equivalents.
      extensions = [
        "${pkgs.rtk}/share/rtk/hooks/pi/rtk.ts"
        ./status-footer.ts
      ];
    };

    context = kit.memories;
  };
}

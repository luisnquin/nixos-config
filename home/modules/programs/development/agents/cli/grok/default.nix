{
  mkAgentKit,
  config,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};

  isAutonomous = (config.agents.permissions.profile or "standard") == "autonomous";

  mcpKeys = [
    "args"
    "command"
    "enabled"
    "env"
    "headers"
    "startup_timeout_sec"
    "tool_timeout_sec"
    "tool_timeouts"
    "url"
  ];

  mcpServers =
    lib.mapAttrs (
      _: server: lib.filterAttrs (name: _: builtins.elem name mcpKeys) server
    ) (kit.mkMcpServers {
      snakeCase = true;
    });

  settings = {
    models.default = "grok-4.6";

    # cli-chat-proxy.grok.com bills against a grok.com subscription we do not
    # have; api.x.ai bills the console team's credits instead. Setting this
    # switches grok to bearer auth and requires XAI_API_KEY in the environment.
    endpoints.models_base_url = "https://api.x.ai/v1";

    ui = {
      simple_mode = false;
      vim_mode = true;
      screen_mode = "fullscreen";
      show_thinking_blocks = true;
      remember_tool_approvals = true;

      default_selected_permission =
        if isAutonomous
        then "always_allow_all_sessions"
        else "allow_once";
    };

    features = {
      telemetry = false;
      feedback = true;
      codebase_indexing = true;
      lsp_tools = true;
    };

    session = {
      auto_compact_threshold_percent = 85;
      load_envrc = true;
    };

    # ~/.cursor/hooks.json uses Cursor's flat command lists, which grok's
    # Claude-shaped matcher parser rejects on every session start.
    compat.cursor.hooks = false;

    mcp_servers = mcpServers;
  };
in {
  # bin/agent collides with cursor-agent, which owns that name in the profile.
  home.packages = [(lib.lowPrio pkgs.llm-agents.grok)];

  # Memories, permissions and skills come from ~/.claude via [compat.claude],
  # enabled by default, so nothing is duplicated under ~/.grok.
  #
  # The TUI rewrites config.toml in place (settings pane, marketplace bootstrap)
  # and would clobber a store symlink, so these land one layer below it, in the
  # only config file grok never writes to.
  home.file.".grok/managed_config.toml".source =
    (pkgs.formats.toml {}).generate "grok-managed-config.toml" settings;
}

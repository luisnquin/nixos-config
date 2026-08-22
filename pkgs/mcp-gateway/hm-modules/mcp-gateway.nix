{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption mkPackageOption types;
  cfg = config.services.mcp-gateway;

  configuredServerType = types.submodule ({config, ...}: {
    options = {
      package = mkOption {
        type = types.package;
        description = "Package containing the MCP server executable.";
      };

      name = mkOption {
        type = types.str;
        default = config.package.passthru.mcp.name or (lib.getName config.package);
        defaultText = "package passthru.mcp.name or package name";
        description = "Gateway subscription name.";
      };

      command = mkOption {
        type = types.str;
        default = config.package.passthru.mcp.command or (lib.getExe config.package);
        defaultText = "package passthru.mcp.command or package main executable";
        description = "MCP server executable.";
      };

      args = mkOption {
        type = types.listOf types.str;
        default = config.package.passthru.mcp.args or [];
        description = "Arguments passed to the MCP server.";
      };

      environment = mkOption {
        type = types.attrsOf types.str;
        default = config.package.passthru.mcp.environment or {};
        description = "Non-secret environment passed to the MCP server.";
      };

      credentials = mkOption {
        type = types.attrsOf types.str;
        default = {};
        description = "Environment variable names mapped to runtime secret files.";
      };

      scope = mkOption {
        type = types.enum ["global" "workspace"];
        default = config.package.passthru.mcp.scope or "global";
        description = "Share one process globally or one per canonical working directory.";
      };

      timeoutSeconds = mkOption {
        type = types.ints.positive;
        default = config.package.passthru.mcp.timeoutSeconds or 120;
        description = "Maximum duration of one upstream request.";
      };

      idleSeconds = mkOption {
        type = types.ints.unsigned;
        default = config.package.passthru.mcp.idleSeconds or 300;
        description = ''
          Workspace process retention after its last client disconnects. With
          reapWhenIdle, also how long the process may go without traffic before
          it is stopped.
        '';
      };

      notifications = mkOption {
        type = types.enum ["drop" "broadcast"];
        default = config.package.passthru.mcp.notifications or "drop";
        description = "Handling for unsolicited server notifications other than progress.";
      };

      startupTimeoutSeconds = mkOption {
        type = types.ints.positive;
        default = 5;
        description = "Client timeout while connecting through the proxy.";
      };

      disabledTools = mkOption {
        type = types.listOf types.str;
        default = config.package.passthru.mcp.disabledTools or [];
        description = "Tool names hidden by clients, carried onto the generated client entries.";
      };

      requires = mkOption {
        type = types.submodule {
          options.anyFileExists = mkOption {
            type = types.listOf types.str;
            default = [];
            description = ''
              Workspace-relative files, any of which must exist for the process
              to be spawned; otherwise clients are served an empty tool list.
              Needs workspace scope.
            '';
          };
        };
        default = {};
        description = "Precondition gating the spawn of a workspace-scoped server.";
      };

      reapWhenIdle = mkOption {
        type = types.bool;
        default = config.package.passthru.mcp.reapWhenIdle or false;
        description = ''
          Stop the shared process after idleSeconds without traffic; the next
          tool call respawns it transparently. Respawning discards the server's
          in-process state, so only opt in servers that can lose it.
        '';
      };
    };
  });

  servers = cfg.servers;
  names = map (server: server.name) servers;
  gatewayConfig = pkgs.writeText "mcp-gateway.json" (builtins.toJSON {
    servers = builtins.listToAttrs (map (server:
      lib.nameValuePair server.name ({
          inherit (server) args command scope;
          env = server.environment;
          credentials = server.credentials;
          timeout_seconds = server.timeoutSeconds;
          idle_seconds = server.idleSeconds;
          notifications = server.notifications;
          reap_when_idle = server.reapWhenIdle;
        }
        // lib.optionalAttrs (server.requires.anyFileExists != []) {
          requires.any_file_exists = server.requires.anyFileExists;
        }))
    servers);
  });
in {
  options.services.mcp-gateway = {
    enable = mkEnableOption "shared local MCP process gateway";

    package = mkPackageOption pkgs "mcp-gateway" {};

    servers = mkOption {
      type = types.listOf (types.coercedTo types.package (package: {inherit package;}) configuredServerType);
      default = [];
      description = "MCP package subscriptions and optional server-specific overrides.";
    };

    protocolVersion = mkOption {
      type = types.enum ["2025-03-26" "2025-06-18"];
      default = "2025-06-18";
      description = ''
        MCP revision spoken to every upstream server, whatever the clients
        negotiate. Clients on either listed revision share one process and are
        answered in their own revision.

        Lower this only for a server that rejects 2025-06-18: the older revision
        cannot carry structuredContent, resource links or completion context, so
        requests and results needing them are refused rather than downgraded
        silently. A server negotiating down to 2024-11-05 is accepted as is,
        since an older upstream emits nothing newer clients cannot read.
        Revisions outside this list are refused at startup; adding one means
        extending pkgs/mcp-gateway/src/compat.rs.
      '';
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      example = "nyx_mcp_gateway=debug";
      description = ''
        RUST_LOG filter for the daemon. The default keeps the protocol
        translation and rejection records, which are the only trace of a client
        being served something other than what the upstream server sent.
      '';
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = builtins.length names == builtins.length (lib.unique names);
        message = "services.mcp-gateway.servers contains duplicate subscription names";
      }
      {
        assertion =
          builtins.all
          (server: server.scope == "workspace" || server.requires.anyFileExists == [])
          servers;
        message = "services.mcp-gateway.servers: requires preconditions need workspace scope";
      }
    ];

    home.packages = [cfg.package];

    programs.mcp = {
      enable = true;
      servers = builtins.listToAttrs (map (server:
        lib.nameValuePair server.name ({
            command = lib.getExe cfg.package;
            args = [server.name];
            startup_timeout_sec = server.startupTimeoutSeconds;
          }
          // lib.optionalAttrs (server.disabledTools != []) {
            inherit (server) disabledTools;
            disabled_tools = server.disabledTools;
          }))
      servers);
    };

    systemd.user.services.mcp-gateway = {
      Unit = {
        Description = "Shared local MCP process gateway";
      };
      Service = {
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe' cfg.package "mcp-gatewayd")
          "--config"
          gatewayConfig
          "--protocol-version"
          cfg.protocolVersion
        ];
        Environment = ["RUST_LOG=${cfg.logLevel}"];
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = ["default.target"];
    };
  };
}

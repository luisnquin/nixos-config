{
  nixosConfig,
  config,
  libx,
  pkgs,
  lib,
}: {
  mkAgentKit = {
    isRoborev ? false,
    audioArgs ? "--volume=32768",
  }: let
    allowedDomains = let
      f = builtins.readFile ./.well-known/ai-allowed-domains.txt;
    in
      builtins.filter (s: s != "") (
        lib.strings.splitString "\n" f
      );

    memories = builtins.readFile ./.well-known/memories.txt;

    permissionLines = let
      f = builtins.readFile ./.well-known/ai-tool-permissions.txt;
      lines = lib.strings.splitString "\n" f;
      cleanLine = line: lib.strings.trim line;
      isRule = line: line != "" && !(lib.hasPrefix "#" line);
    in
      builtins.filter isRule (map cleanLine lines);

    parsedPermissions = map (line: let
      sign = builtins.substring 0 1 line;
      rule = lib.strings.trim (
        builtins.substring 1 ((builtins.stringLength line) - 1) line
      );
    in
      if sign == "+"
      then {
        action = "allow";
        inherit rule;
      }
      else if sign == "-"
      then {
        action = "deny";
        inherit rule;
      }
      else if sign == "?"
      then {
        action = "ask";
        inherit rule;
      }
      else throw "Invalid agent tool permission line: ${line}")
    permissionLines;

    toolPermissions = {
      allow = map (entry: entry.rule) (
        builtins.filter (entry: entry.action == "allow") parsedPermissions
      );
      deny = map (entry: entry.rule) (
        builtins.filter (entry: entry.action == "deny") parsedPermissions
      );
      ask = map (entry: entry.rule) (
        builtins.filter (entry: entry.action == "ask") parsedPermissions
      );
    };

    translateGeminiRule = rule: let
      toolMap = {
        "Glob" = "glob";
        "Grep" = "grep_search";
        "LS" = "list_directory";
        "Read" = "read_file";
        "Search" = "grep_search";
        "Bash" = "run_shell_command";
        "Write" = "write_file";
        "Edit" = "replace";
        "Task" = "write_todos";
        "TodoWrite" = "write_todos";
      };

      match = builtins.match "([A-Za-z]+)\\((.*)\\)" rule;
    in
      if match != null
      then let
        tool = builtins.elemAt match 0;
        args = builtins.elemAt match 1;
        geminiTool = toolMap.${tool} or tool;
      in
        if args == "*"
        then geminiTool
        else "${geminiTool}(${args})"
      else rule;

    mkAgentPermissions = target: let
      # We check if we're in autonomous mode to adjust defaults
      isAutonomous = (config.agents.permissions.profile or "standard") == "autonomous";
    in
      if target == "claude"
      then {
        allow = toolPermissions.allow ++ builtins.map (d: "WebFetch(domain:${d})") allowedDomains;
        ask = toolPermissions.ask;
        deny = toolPermissions.deny;
        defaultMode =
          if isAutonomous
          then "acceptEdits"
          else "default";
        disableBypassPermissionsMode = "disable";
      }
      else if target == "gemini"
      then {
        allowed = map translateGeminiRule toolPermissions.allow;
        confirmationRequired = map translateGeminiRule toolPermissions.ask;
        exclude = map translateGeminiRule toolPermissions.deny;
      }
      else if target == "opencode"
      then let
        status = tool:
          if builtins.any (r: lib.hasPrefix "${tool}" r) toolPermissions.allow
          then "allow"
          else if builtins.any (r: lib.hasPrefix "${tool}" r) toolPermissions.ask
          then "ask"
          else "deny";
      in {
        read = status "Read";
        glob = status "Glob";
        grep = status "Grep";
        list = status "LS";
        edit = status "Edit";
        write = status "Write";
        bash = status "Bash";
        webfetch = status "WebFetch";
        websearch = status "Search";
      }
      else if target == "codex"
      then {
        # Restore the safer logic for Codex
        approval_policy =
          if isAutonomous
          then "never"
          else "untrusted";
        sandbox_mode =
          if isAutonomous
          then "danger-full-access"
          else "workspace-write";
        web_search = {
          context_size = "medium";
          allowed_domains = allowedDomains;
        };
      }
      else throw "Unsupported agent permission target: ${target}";

    audioArgsPart = lib.optionalString (audioArgs != "") "${audioArgs} ";

    guardRoborev = command:
      if isRoborev
      then ''
        if [ -z "$ROBOREV" ]; then
          ${command}
        fi
      ''
      else command;

    audioCommand = file: "${pkgs.pulseaudio}/bin/paplay ${audioArgsPart}${lib.escapeShellArg file}";

    notificationCommand = image: title: message:
      lib.concatStringsSep " " [
        (lib.getExe pkgs.libnotify)
        "-a"
        (lib.escapeShellArg title)
        "-i"
        (lib.escapeShellArg image)
        (lib.escapeShellArg title)
        (lib.escapeShellArg message)
      ];
  in rec {
    inherit (import ./assets {inherit lib;}) sounds images;
    inherit memories allowedDomains;
    inherit mkAgentPermissions;

    mkAudioCmd = files:
      guardRoborev (
        builtins.concatStringsSep " && " (map audioCommand files)
      );

    mkNotificationCmd = image: title: message: {ntfy ? {}}: let
      ntfyPart = libx.comms.mkNtfy nixosConfig.services.ntfy-sh.settings.base-url ({
          topic = "agents";
          inherit title message;
        }
        // ntfy);
      base = notificationCommand image title message;
    in
      guardRoborev "(${base}) && (${ntfyPart})";

    mkCancelNotificationCmd = {
      topic ? "agents",
      sequenceId,
    }:
      guardRoborev (
        libx.comms.cancelNtfy nixosConfig.services.ntfy-sh.settings.base-url {
          inherit topic sequenceId;
        }
      );

    mkCmdEntry = {
      matcher ? null,
      commands,
    }:
      {hooks = map (command: {
        type = "command";
        inherit command;
      }) commands;}
      // lib.optionalAttrs (matcher != null) {
        inherit matcher;
      };

    mkMcpServers = let
      inherit (config.programs.mcp) servers;
    in
      {
        excludeServers ? [],
        excludeTools ? {},
        snakeCase ? false,
        normalizeServerUrl ? false,
      }: let
        applyToolExclusions = name: server: let
          excluded = excludeTools.${name} or [];
          disabledTools = server.disabledTools or [];
          nextDisabledTools = lib.unique (disabledTools ++ excluded);
        in
          server
          // lib.optionalAttrs (nextDisabledTools != []) {
            disabledTools = nextDisabledTools;
          };

        applyMappings = server:
          server
          // lib.optionalAttrs snakeCase (
            lib.optionalAttrs (server ? disabledTools) {
              disabled_tools = server.disabledTools;
            }
          )
          // lib.optionalAttrs normalizeServerUrl (
            lib.optionalAttrs (server ? url && !(server ? serverUrl)) {
              serverUrl = server.url;
            }
          );

        transformServer = name: server:
          applyMappings (applyToolExclusions name server);
      in
        lib.mapAttrs transformServer (
          builtins.removeAttrs servers excludeServers
        );
  };
}

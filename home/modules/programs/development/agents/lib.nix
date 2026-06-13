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

      baseLines = builtins.filter isRule (map cleanLine lines);
      isBashRule = line:
        builtins.match "[+?-][[:space:]]+Bash\\(.*\\)" line != null;

      isRtkBashRule = line:
        builtins.match "[+?-][[:space:]]+Bash\\(rtk .*\\)" line != null;
      mkRtkBashRule = line: let
        sign = builtins.substring 0 1 line;
        body = lib.strings.trim (
          builtins.substring 1 ((builtins.stringLength line) - 1) line
        );

        match = builtins.match "Bash\\((.*)\\)" body;
        command = builtins.elemAt match 0;
      in "${sign} Bash(rtk ${command})";

      expandLine = line:
        if isBashRule line && !(isRtkBashRule line)
        then [line (mkRtkBashRule line)]
        else [line];
    in
      lib.unique (lib.flatten (map expandLine baseLines));

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

    translateAntigravityRule = rule: let
      match = builtins.match "([A-Za-z_][A-Za-z0-9_]*)\\((.*)\\)" rule;
    in
      if match != null
      then let
        tool = builtins.elemAt match 0;
        args = builtins.elemAt match 1;
        cleanArgs = lib.removeSuffix ":*" args;
      in
        if tool == "Bash"
        then "command(${cleanArgs})"
        else if lib.hasPrefix "mcp__" tool
        then "mcp(${lib.removePrefix "mcp__" tool})"
        else if tool == "WebFetch"
        then
          if args == "*"
          then "read_url(*)"
          else "read_url(${args})"
        else if args == "*" || args == ""
        then lib.toLower tool
        else "${lib.toLower tool}(${cleanArgs})"
      else if lib.hasPrefix "mcp__" rule
      then "mcp(*)"
      else lib.toLower rule;

    mkAgentPermissions = target: extra: let
      extraAllow = extra.allow or [];
      extraAsk = extra.ask or [];
      extraDeny = extra.deny or [];

      # We check if we're in autonomous mode to adjust defaults
      isAutonomous = (config.agents.permissions.profile or "standard") == "autonomous";
    in
      if target == "claude"
      then {
        allow =
          toolPermissions.allow
          ++ builtins.map (d: "WebFetch(domain:${d})") allowedDomains
          ++ extraAllow;

        ask = toolPermissions.ask ++ extraAsk;
        deny = toolPermissions.deny ++ extraDeny;

        defaultMode =
          if isAutonomous
          then "acceptEdits"
          else "default";

        disableBypassPermissionsMode = "disable";
      }
      else if target == "antigravity"
      then {
        allow =
          map translateAntigravityRule (toolPermissions.allow ++ extraAllow)
          ++ builtins.map (d: "read_url(${d})") allowedDomains;

        ask = map translateAntigravityRule (toolPermissions.ask ++ extraAsk);
        deny = map translateAntigravityRule (toolPermissions.deny ++ extraDeny);
      }
      else if target == "opencode"
      then let
        mergedPermissions = {
          allow = toolPermissions.allow ++ extraAllow;
          ask = toolPermissions.ask ++ extraAsk;
          deny = toolPermissions.deny ++ extraDeny;
        };

        status = tool:
          if builtins.any (r: lib.hasPrefix "${tool}" r) mergedPermissions.allow
          then "allow"
          else if builtins.any (r: lib.hasPrefix "${tool}" r) mergedPermissions.ask
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

    agentNotify = pkgs.writeShellApplication {
      name = "agent-notify";
      runtimeInputs = [pkgs.curl pkgs.systemd pkgs.libnotify pkgs.coreutils];
      text = builtins.readFile ./notify/agent-notify.sh;
    };
  in {
    inherit (import ./assets {inherit lib;}) sounds images;
    inherit memories allowedDomains;
    inherit mkAgentPermissions;

    mkAudioCmd = files:
      guardRoborev (
        builtins.concatStringsSep " && " (map audioCommand files)
      );

    # A delay + sequenceId means the notification must be cancelable, so it is
    # held in a local systemd timer via agent-notify. Without them it fires
    # immediately through libx.notify (desktop + ntfy), unchanged.
    mkNotificationCmd = image: title: message: {ntfy ? {}}: let
      isScheduled = (ntfy ? delay) && (ntfy ? sequenceId);

      host = nixosConfig.services.ntfy-sh.settings.base-url;
      topic = ntfy.topic or "agents";
      ntfyUrl =
        if host == null || host == ""
        then ""
        else "${lib.removeSuffix "/" host}/${topic}";
    in
      guardRoborev (
        if isScheduled
        then
          lib.concatStringsSep " " (
            [
              (lib.getExe agentNotify)
              "schedule"
              "--id"
              (lib.escapeShellArg ntfy.sequenceId)
              "--delay"
              (lib.escapeShellArg ntfy.delay)
              "--title"
              (lib.escapeShellArg title)
              "--message"
              (lib.escapeShellArg message)
              "--image"
              (lib.escapeShellArg image)
            ]
            ++ lib.optionals (ntfyUrl != "") [
              "--ntfy-url"
              (lib.escapeShellArg ntfyUrl)
            ]
          )
        else
          libx.notify.send {
            desktop = {
              inherit image title message;
            };
            ntfy =
              {
                inherit host topic;
              }
              // builtins.removeAttrs ntfy ["delay" "sequenceId"];
          }
      );

    mkCancelNotificationCmd = {sequenceId, ...}:
      guardRoborev (
        lib.concatStringsSep " " [
          (lib.getExe agentNotify)
          "cancel"
          "--id"
          (lib.escapeShellArg sequenceId)
        ]
      );

    mkCmdEntry = {
      matcher ? null,
      commands,
    }:
      {
        hooks =
          map (command: {
            type = "command";
            inherit command;
          })
          commands;
      }
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

        cleanServer = server:
          lib.filterAttrs (_: value: value != null && value != [] && value != {}) server;

        transformServer = name: server:
          cleanServer (applyMappings (applyToolExclusions name server));
      in
        lib.mapAttrs transformServer (
          builtins.removeAttrs servers excludeServers
        );
  };
}

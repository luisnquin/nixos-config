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
      isBashPrefix = line:
        builtins.match "@[[:space:]]+BashPrefix\\(.*\\)" line != null;

      parseBashPrefix = line: let
        match = builtins.match "@[[:space:]]+BashPrefix\\((.*)\\)" line;
      in
        if match != null
        then builtins.elemAt match 0
        else throw "Invalid agent Bash prefix line: ${line}";

      bashPrefixes = map parseBashPrefix (
        builtins.filter isBashPrefix baseLines
      );

      permissionSourceLines = builtins.filter (line: !(isBashPrefix line)) baseLines;

      mkPrefixedBashRule = prefix: line: let
        sign = builtins.substring 0 1 line;
        body = lib.strings.trim (
          builtins.substring 1 ((builtins.stringLength line) - 1) line
        );

        match = builtins.match "Bash\\((.*)\\)" body;
        command = builtins.elemAt match 0;
        prefixedCommand = "${prefix} ${command}";
      in "${sign} Bash(${prefixedCommand})";

      expandLine = line:
        if isBashRule line
        then [line] ++ map (prefix: mkPrefixedBashRule prefix line) bashPrefixes
        else [line];
    in
      lib.unique (lib.flatten (map expandLine permissionSourceLines));

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

        # disableBypassPermissionsMode = "disable";
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
      else if target == "grok"
      then {}
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

    agentTerminalStatus = pkgs.writeShellApplication {
      name = "agent-terminal-status";
      text = builtins.readFile ../../terminal/scripts/agent-terminal-status.sh;
    };

    # The scripts shell out to python3, which a hook's inherited PATH does not
    # guarantee.
    herdrSessionHook = agent:
      pkgs.runCommandLocal "herdr-agent-state-${agent}" {
        nativeBuildInputs = [pkgs.makeWrapper];
      } ''
        install -Dm755 \
          ${pkgs.llm-agents.herdr.src}/src/integration/assets/${agent}/herdr-agent-state.sh \
          $out/libexec/herdr-agent-state
        makeWrapper $out/libexec/herdr-agent-state $out/bin/herdr-agent-state \
          --prefix PATH : ${lib.makeBinPath [pkgs.python3]}
      '';
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

    mkTerminalStatusCmd = state: title:
      guardRoborev (
        lib.concatStringsSep " " [
          (lib.getExe agentTerminalStatus)
          (lib.escapeShellArg state)
          (lib.escapeShellArg title)
        ]
      );

    # herdr learns a pane's agent session id only from this script, and it
    # installs the script by rewriting the agent's own settings file. Those
    # files belong to home-manager, so every rebuild dropped herdr's entry and
    # `session.resume_agents_on_restore` had nothing to resume. Ship the script
    # from the flake input instead and wire it beside the other hooks.
    mkHerdrSessionCmd = agent: "${herdrSessionHook agent}/bin/herdr-agent-state session";

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
          lib.hm.mcp.addType server
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

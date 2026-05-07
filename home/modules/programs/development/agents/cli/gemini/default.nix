{
  mkAgentKit,
  config,
  pkgs,
  ...
}: let
  kit = mkAgentKit {};

  rtkPath = ".gemini/hooks/rtk-hook-gemini.sh";

  geminiTools = kit.mkAgentPermissions "gemini";

  mkPolicy = decision: rule: let
    match = builtins.match "([A-Za-z_]+)\\((.*)\\)" rule;
    toolName = if match != null then builtins.elemAt match 0 else rule;
    args = if match != null then builtins.elemAt match 1 else "*";
    hasArgs = args != "*";
    prefixRaw = pkgs.lib.removeSuffix "*" args;
  in ''
    [[rule]]
    toolName = "${toolName}"
    decision = "${if decision == "ask" then "ask_user" else decision}"
    priority = ${if decision == "deny" then "200" else "100"}
  '' + pkgs.lib.optionalString (hasArgs && toolName == "run_shell_command") ''
    commandPrefix = "${prefixRaw}"
  '';

  policyToml = pkgs.lib.concatStringsSep "\n" (
    (map (mkPolicy "allow") geminiTools.allowed) ++
    (map (mkPolicy "ask") geminiTools.confirmationRequired) ++
    (map (mkPolicy "deny") geminiTools.exclude)
  );
in {
  home.file = {
    "${rtkPath}" = {
      text = ''
        #!/bin/sh
        exec rtk hook gemini
      '';
      executable = true;
    };
    ".gemini/policies/nix-rules.toml".text = policyToml;

    ".gemini/.env".text = ''
      PINENTRY_USER_DATA=gui
    '';
  };

  programs.gemini-cli = {
    enable = true;
    package = pkgs.llm-agents.gemini-cli;
    defaultModel = "gemini-2.5-flash";

    context = {
      GEMINI = kit.memories;
    };

    settings = {
      theme = "Default";
      vimMode = true;
      preferredEditor = "nano";
      autoAccept = false;
      secureModeEnabled = false;
      security.auth.selectedType = "oauth-personal";

      hooks = {
        SessionStart = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifarm];
          })
        ];

        SessionEnd = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifdarm];
          })
        ];

        BeforeAgent = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifrsig];
          })
        ];

        PreCompress = [
          (kit.mkAudioEntry {
            files = [kit.sounds.ifrsig];
          })
        ];

        Notification = [
          (kit.mkNotificationEntry {
            image = kit.images.gemini;
            title = "Gemini";
            message = "Awaiting your input";
            extraHooks = [
              (kit.mkAudioHook [kit.sounds.buzact])
            ];
          })
        ];

        BeforeTool = [
          {
            matcher = "run_shell_command";
            hooks = [
              {
                type = "command";
                command = "${config.home.homeDirectory}/${rtkPath}";
              }
            ];
          }
        ];
      };
    };
  };
}

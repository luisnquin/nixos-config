{
  mkAgentKit,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};

  rtkPath = ".gemini/hooks/rtk-hook-gemini.sh";

  antigravityPermissions = kit.mkAgentPermissions "antigravity" {};
in {
  imports = [
    ./hooks.nix
  ];

  home.file = {
    "${rtkPath}" = {
      text = ''
        #!/bin/sh
        exec rtk hook gemini
      '';
      executable = true;
    };

    ".gemini/.env".text = ''
      PINENTRY_USER_DATA=gui
    '';
  };

  programs.antigravity-cli = {
    enable = true;
    package = lib.lowPrio pkgs.llm-agents.antigravity-cli;
    enableMcpIntegration = true;
    defaultModel = "gemini-2.5-flash";

    context = {
      GEMINI = kit.memories;
    };

    permissions = {
      allow = antigravityPermissions.allow;
      ask = antigravityPermissions.ask;
      deny = antigravityPermissions.deny;
    };

    settings = {
      vimMode = true;
      preferredEditor = "nano";
    };
  };
}

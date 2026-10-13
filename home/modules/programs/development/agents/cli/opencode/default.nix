{
  pkgs,
  mkAgentKit,
  ...
}: let
  kit = mkAgentKit {};
in {
  # https://opencode.ai/config.json
  programs.opencode = {
    enable = true;
    package = pkgs.llm-agents.opencode;

    plugins = {
      "rtk" = "${pkgs.rtk}/share/rtk/hooks/opencode-rtk.ts";
    };

    settings = {
      server.port = 4096;

      permission = kit.mkAgentPermissions "opencode" {};

      snapshot = true;
      autoupdate = false;
    };
  };
}

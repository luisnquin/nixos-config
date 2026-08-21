{
  config,
  pkgs,
  lib,
  ...
}: {
  home.packages = [pkgs.codebase-memory-mcp];

  programs.techdebt.enable = true;

  services.mcp-gateway = {
    enable = true;
    servers = [
      pkgs.context7-mcp
      pkgs.mcp-nixos
      pkgs.mcp-server-sequential-thinking
      {
        name = "filesystem";
        package = pkgs.mcp-server-filesystem;
        args = [
          "."
          "/tmp"
        ];
        scope = "workspace";
      }
      {
        name = "techdebt-mcp";
        package = config.programs.techdebt.package;
        args = ["mcp"];
        scope = "workspace";
      }
    ];
  };

  programs.mcp = {
    enable = true;
    servers = {
      codebase-memory-mcp.command = lib.getExe pkgs.codebase-memory-mcp;

      firefox-devtools = {
        command = lib.getExe pkgs.firefox-devtools-mcp;
        args = [
          "--connectExisting"
          "--marionettePort"
          "2828"
        ];
      };

      encore = rec {
        command = "encore";
        args = ["mcp" "run" "--app=gate-k9-mzni"];
        disabledTools = [
          "query_database"
          "get_secrets"
        ];
        disabled_tools = disabledTools;
      };

      linear.url = "https://mcp.linear.app/mcp";
    };
  };
}

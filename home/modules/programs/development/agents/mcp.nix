{
  config,
  pkgs,
  lib,
  inputs,
  system,
  ...
}: let
  encore = inputs.encore.packages.${system}.encore;
in {
  home.packages = [pkgs.codebase-memory-mcp];

  programs.techdebt.enable = true;

  services.mcp-gateway = {
    enable = true;
    servers = [
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
      {
        name = "encore";
        package = encore;
        command = lib.getExe' encore "encore";
        args = ["mcp" "run"];
        scope = "workspace";
        requires.anyFileExists = ["encore.app"];
        disabledTools = [
          "query_database"
          "get_secrets"
        ];
      }
      {
        name = "firefox-devtools";
        package = pkgs.firefox-devtools-mcp;
        args = [
          "--connectExisting"
          "--marionettePort"
          "2828"
        ];
        reapWhenIdle = true;
      }
    ];
  };

  programs.mcp = {
    enable = true;
    servers = {
      codebase-memory-mcp.command = lib.getExe pkgs.codebase-memory-mcp;

      context7.url = "https://mcp.context7.com/mcp";

      linear.url = "https://mcp.linear.app/mcp";
    };
  };
}

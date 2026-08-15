{
  config,
  pkgs,
  lib,
  ...
}: {
  home.packages = [pkgs.codebase-memory-mcp];

  programs.techdebt.enable = true;

  programs.mcp = {
    enable = true;
    servers = {
      adb.command = lib.getExe pkgs.adb-mcp;

      codebase-memory-mcp.command = lib.getExe pkgs.codebase-memory-mcp;

      context7.command = lib.getExe pkgs.context7-mcp;

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

      filesystem = {
        command = lib.getExe pkgs.mcp-server-filesystem;
        args = [
          "."
          "/tmp"
        ];
        startup_timeout_sec = 5;
      };

      linear.url = "https://mcp.linear.app/mcp";

      nixos.command = lib.getExe pkgs.mcp-nixos;

      sequential-thinking.command = lib.getExe pkgs.mcp-server-sequential-thinking;

      techdebt-mcp = {
        command = lib.getExe config.programs.techdebt.package;
        args = ["mcp"];
      };

      # supabase = rec {
      #   url = "https://mcp.supabase.com/mcp?project_ref=mjkvxcziwkwohxpuejak";
      #   disabledTools = [
      #     "reset_branch"
      #     "rebase_branch"
      #     "delete_branch"
      #     "merge_branch"
      #     "list_migrations"
      #     "apply_migration"
      #     "list_branches"
      #     "create_branch"
      #     "deploy_edge_function"
      #     "get_edge_function"
      #     "list_edge_functions"
      #     "execute_sql"
      #   ];
      #   disabled_tools = disabledTools;
      # };
    };
  };
}

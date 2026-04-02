{
  inputs,
  system,
  pkgs,
  lib,
  ...
}: {
  programs.mcp = {
    enable = true;
    servers = {
      adb = {
        type = "stdio";
        command = lib.getExe inputs.adb-mcp.packages.${system}.default;
      };

      context7 = {
        type = "stdio";
        command = lib.getExe pkgs.context7-mcp;
      };

      encore = {
        type = "stdio";
        command = "encore";
        args = ["mcp" "run" "--app=gate-k9-mzni"];
        disabledTools = [
          "query_database"
          "get_secrets"
        ];
      };

      filesystem = {
        type = "stdio";
        command = "npx";
        args = [
          "-y"
          "@modelcontextprotocol/server-filesystem"
          "/tmp"
        ];
        disabledTools = [
          "move_file"
          "list_allowed_directories"
          "list_directory"
        ];
      };

      # github.command = lib.getExe pkgs.github-mcp-server;
      nixos = {
        type = "stdio";
        command = lib.getExe pkgs.mcp-nixos;
      };

      supabase = {
        type = "http";
        url = "https://mcp.supabase.com/mcp?project_ref=mjkvxcziwkwohxpuejak";
        disabledTools = [
          "reset_branch"
          "rebase_branch"
          "delete_branch"
          "merge_branch"
          "list_migrations"
          "apply_migration"
          "list_branches"
          "create_branch"
          "deploy_edge_function"
          "get_edge_function"
          "list_edge_functions"
          "execute_sql"
        ];
      };
    };
  };
}

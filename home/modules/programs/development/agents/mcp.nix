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
      adb.command = lib.getExe inputs.adb-mcp.packages.${system}.default;
      context7.command = lib.getExe pkgs.context7-mcp;

      encore = {
        command = "encore";
        args = ["mcp" "run" "--app=gate-k9-mzni"];
        disabledTools = [
          "query_database"
          "get_secrets"
        ];
      };

      github.command = lib.getExe pkgs.github-mcp-server;
      nixos.command = lib.getExe pkgs.mcp-nixos;

      supabase = {
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

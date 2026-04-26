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

      encore = rec {
        type = "stdio";
        command = "encore";
        args = ["mcp" "run" "--app=gate-k9-mzni"];
        disabledTools = [
          "query_database"
          "get_secrets"
        ];
        disabled_tools = disabledTools;
      };

      filesystem = rec {
        type = "stdio";
        command = lib.getExe pkgs.mcp-server-filesystem;
        args = [
          "/tmp"
        ];
        disabledTools = [
          "move_file"
          "list_allowed_directories"
          "list_directory"
        ];
        disabled_tools = disabledTools;
        startup_timeout_sec = 5;
      };

      # github.command = lib.getExe pkgs.github-mcp-server;
      # nixos = {
      #   type = "stdio";
      #   command = lib.getExe pkgs.mcp-nixos;
      # };

      # supabase = rec {
      #   type = "http";
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

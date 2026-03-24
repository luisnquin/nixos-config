{
  programs.mcp = {
    enable = true;
    servers = {
      encore = {
        command = "encore";
        args = ["mcp" "run" "--app=gate-k9-mzni"];
        disabledTools = [
          "query_database"
          "get_secrets"
        ];
      };

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

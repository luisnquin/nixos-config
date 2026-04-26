{
  mkAgentKit,
  pkgs,
  ...
}: let
  kit = mkAgentKit {
    isRoborev = true;
  };
in {
  programs.roborev = {
    enable = true;
    env = {
      ROBOREV_COLOR_MODE = "dark";
      ROBOREV = "1";
    };

    settings = {
      server_addr = "127.0.0.1:7373";
      max_workers = 4;
      review_context_count = 3;
      reuse_review_session_lookback = 0;
      # Default agent when no workflow-specific agent is set.
      default_agent = "gemini";
      default_model = "";
      default_backup_agent = "";
      default_backup_model = "";
      job_timeout_minutes = 30;
      # Default reasoning level for reviews: fast, standard, medium, thorough, or maximum.
      review_reasoning = "";
      # Default reasoning level for refine: fast, standard, medium, thorough, or maximum.
      refine_reasoning = "";
      # Default reasoning level for fix: fast, standard, medium, thorough, or maximum.
      fix_reasoning = "";

      review_agent = "";
      review_agent_fast = "";
      review_agent_standard = "";
      review_agent_medium = "";
      review_agent_thorough = "";
      review_agent_maximum = "";

      refine_agent = "";
      refine_agent_fast = "";
      refine_agent_standard = "";
      refine_agent_medium = "";
      refine_agent_thorough = "";
      refine_agent_maximum = "";

      review_model = "";
      review_model_fast = "";
      review_model_standard = "";
      review_model_medium = "";
      review_model_thorough = "";
      review_model_maximum = "";

      refine_model = "";
      refine_model_fast = "";
      refine_model_standard = "";
      refine_model_medium = "";
      refine_model_thorough = "";
      refine_model_maximum = "";

      fix_agent = "";
      fix_agent_fast = "";
      fix_agent_standard = "";
      fix_agent_medium = "";
      fix_agent_thorough = "";
      fix_agent_maximum = "";

      fix_model = "";
      fix_model_fast = "";
      fix_model_standard = "";
      fix_model_medium = "";
      fix_model_thorough = "";
      fix_model_maximum = "";

      security_agent = "";
      security_agent_fast = "";
      security_agent_standard = "";
      security_agent_medium = "";
      security_agent_thorough = "";
      security_agent_maximum = "";

      security_model = "";
      security_model_fast = "";
      security_model_standard = "";
      security_model_medium = "";
      security_model_thorough = "";
      security_model_maximum = "";

      design_agent = "";
      design_agent_fast = "";
      design_agent_standard = "";
      design_agent_medium = "";
      design_agent_thorough = "";
      design_agent_maximum = "";

      design_model = "";
      design_model_fast = "";
      design_model_standard = "";
      design_model_medium = "";
      design_model_thorough = "";
      design_model_maximum = "";

      review_backup_agent = "";
      refine_backup_agent = "";
      fix_backup_agent = "";
      security_backup_agent = "";
      design_backup_agent = "";

      review_backup_model = "";
      refine_backup_model = "";
      fix_backup_model = "";
      security_backup_model = "";
      design_backup_model = "";

      # Minimum severity for reviews: critical, high, medium, or low. Empty disables filtering.
      review_min_severity = "";
      # Minimum severity for refine: critical, high, medium, or low. Empty disables filtering.
      refine_min_severity = "";
      # Minimum severity for fix: critical, high, medium, or low. Empty disables filtering.
      fix_min_severity = "";

      disable_codex_sandbox = false;
      codex_cmd = "${pkgs.coreutils}/bin/false";
      claude_code_cmd = "${pkgs.coreutils}/bin/false";
      cursor_cmd = "${pkgs.coreutils}/bin/false";
      pi_cmd = "${pkgs.coreutils}/bin/false";
      opencode_cmd = "opencode";

      hooks = [
        {
          event = "review.completed";
          command = kit.mkNotificationCmd kit.images.roborev "Review completed" "[{repo_name}] Review done for {sha}: {verdict}";
        }
        {
          event = "review.failed";
          command = kit.mkNotificationCmd kit.images.roborev "Review failed" "[{repo_name}] Error on {sha}: run roborev show {job_id}";
        }
      ];

      # Filenames or glob patterns to exclude from review diffs globally.
      exclude_patterns = [];

      default_max_prompt_size = 0;

      # Automatically close reviews that pass with no findings.
      auto_close_passing_reviews = false;
      # Hide closed reviews by default in the TUI queue.
      hide_closed_by_default = false;
      hide_addressed_by_default = false;
      # Automatically filter the TUI queue to the current repo.
      auto_filter_repo = false;
      # Automatically filter the TUI queue to the current branch.
      auto_filter_branch = false;
      # Enable mouse support in the TUI.
      mouse_enabled = true;
      tab_width = 0;
      # Queue columns to hide in the TUI.
      hidden_columns = ["_"];
      # Show column borders in the TUI queue.
      column_borders = false;
      # Custom queue column order in the TUI.
      column_order = [];
      # Custom Tasks column order in the TUI.
      task_column_order = [];
      column_config_version = 0;

      sync = {
        enabled = false;
        postgres_url = "";
        interval = "";
        machine_name = "";
        connect_timeout = "";
      };

      ci = {
        enabled = false;
        poll_interval = "";
        repos = [];
        exclude_repos = [];
        max_repos = 0;
        review_types = [];
        agents = [];
        throttle_interval = "";
        throttle_bypass_users = ["wesm" "mariusvniekerk"];
        model = "";
        synthesis_agent = "";
        synthesis_backup_agent = "";
        synthesis_model = "";
        min_severity = "";
        upsert_comments = false;
        batch_timeout = "";
        github_app_id = 0;
        github_app_private_key = "";
        github_app_installation_id = 0;
      };

      advanced = {
        # Enable the advanced Tasks workflow in the TUI.
        tasks_enabled = false;
      };
    };
  };
}

{config, ...}: {
  programs.codex = {
    enable = true;
    enableMcpIntegration = true;

    custom-instructions = config.programs.claude-code.settings.memory.text;

    settings = {
      analytics.enabled = true;
      feedback.enabled = true;
      mcp_servers = builtins.removeAttrs config.programs.mcp.servers [
        "supabase"
      ];

      agents = {
        job_max_runtime_seconds = 3600;
        max_depth = 5;
        max_threads = 10;
      };

      approval_policy = "untrusted";
      approvals_reviewer = "user";
      sandbox_mode = "workspace-write";

      profiles = {
        coding = {
          model = "gpt-4-turbo";
          personality = "pragmatic";
          features = {
            code_mode = true;
            apply_patch_freeform = true;
          };
        };
        creative = {
          personality = "friendly";
          model_verbosity = "high";
        };
      };

      tools = {
        view_image = true;
        web_search = {
          context_size = "medium";
          allowed_domains = ["github.com" "stackoverflow.com"];
        };
      };
    };
  };
}

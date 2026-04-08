{pkgs, ...}: {
  programs.agents = {
    enable = true;

    defaultScopes = ["common" "claude"];

    skills = with pkgs.agent-skills; [
      (official.encoredev.skills {
        plugins = [
          "encore-api"
          "encore-code-review"
          "encore-service"
          "encore-auth"
          "encore-database"
          "encore-testing"
        ];
      })
      (official.anthropics.skills {
        prefix = "anthropics-";
        plugins = [
          "pdf"
          "pptx"
          "frontend-design"
        ];
      })
      (unofficial.daffy0208.ai-dev-standards {
        plugins = [
          "mobile-developer"
        ];
      })
      (unofficial.wshobson.agents {
        plugins = [
          "typescript-advanced-types"
          "e2e-testing-patterns"
        ];
      })
    ];
  };
}

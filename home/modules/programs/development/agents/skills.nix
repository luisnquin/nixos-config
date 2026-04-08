{pkgs, ...}: {
  programs.agents = {
    enable = true;

    skills = with pkgs.agent-skills; [
      (official.encoredev.skills {
        scopes = ["claude" "global"];
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
        scopes = ["global"];
        plugins = [
          "pdf"
          "pptx"
        ];
      })
    ];
  };
}

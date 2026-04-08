{
  inputs,
  pkgs,
  ...
}: let
  inherit (inputs.agentic-flake.lib) mkSkill;
in {
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
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "aia-11-hn-mib";
            repo = "mib-mockinterviewaibot";
            rev = "50ccfb29063bb6d64d049fc982ca53424d0ca3b1";
            sha256 = "sha256-FdlbFVFSEtiFMiuisSLmgibFCro17jo3TjS9Oibx8F0=";
          };
        } {
          plugins = ["imagemagick"];
        })
    ];
  };
}

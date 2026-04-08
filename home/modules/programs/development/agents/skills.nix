{
  inputs,
  pkgs,
  ...
}: let
  inherit (inputs.agentic-flake.lib) mkSkill;
in {
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
          "frontend-design"
        ];
      })
      (unofficial.daffy0208.ai-dev-standards {
        scopes = ["global" "claude"];
        plugins = [
          "mobile-developer"
        ];
      })
      ((mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "wshobson";
            repo = "agents";
            rev = "70444e5b1fae2237f3cb087c70db043ab633fe11";
            sha256 = "sha256-2D47P2shN4etixbN92/VPiZm90q6AizHbV1M/mLJC4s=";
          };
        }) {
          scopes = ["global" "claude"];
          plugins = [
            "typescript-advanced-types"
            "e2e-testing-patterns"
          ];
        })
    ];
  };
}

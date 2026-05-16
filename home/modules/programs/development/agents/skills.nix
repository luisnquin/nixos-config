{
  inputs,
  pkgs,
  ...
}: let
  inherit (inputs.agentic-flake.lib) mkSkill mkInlineSkill;
in {
  programs.agents = {
    enable = true;

    defaultScopes = ["common" "claude"];

    skills = with pkgs.agent-skills; [
      (mkInlineSkill {
        "nixgrep" = {
          description = "Search nix derivations from the /nix/store";
          tags = ["utils"];
          content = ''
            # Quick Reference

            $ nixgrep nao
            q0bfqrchwnip1n48idwmsblrqx3rds37-nao
            bsia93f0mzdrrdxigdmc48i9zcg2j4j7-nao-3.3.0
            298vl82bfm4939fd0clmw1m1hzid675j-nao-3.3.0-go-modules
            7gr2qa7saimqvnhl8nwgvlaf9fid25g9-nao-3.3.0-go-modules.drv
            rwajk54dm1wa3iqg0mahpv7bxhjyapfk-nao-3.3.0-go-modules.drv
          '';
        };
      } {
        plugins = ["nixgrep"];
      })
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

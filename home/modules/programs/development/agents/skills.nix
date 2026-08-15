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
      (mkInlineSkill {
          "phone" = {
            description = "Drive the Android handsets and emulators on this desk with the `phone` CLI — pick a device, screenshot it, list the elements on screen, tap by name or coordinate, type, send keys, and handle split screen focus and foldable displays. Use for any hands-on Android device automation from this host.";
            tags = ["mobile"];
            content = builtins.readFile ./skills/phone.md;
          };
        } {
          plugins = ["phone"];
        })
      (anthropics.skills {
        prefix = "anthropics-";
        plugins = [
          "pdf"
          "pptx"
          "frontend-design"
        ];
      })
      (daffy0208.ai-dev-standards {
        plugins = [
          "mobile-developer"
        ];
      })
      (vercel-labs.skills {
        plugins = [
          "find-skills"
        ];
      })
      (wshobson.agents {
        plugins = [
          "typescript-advanced-types"
          "e2e-testing-patterns"
        ];
      })
      (mkSkill {src = inputs.herdr;} {
        plugins = ["herdr"];
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
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "0xc000022070";
            repo = "skills";
            rev = "a6a17b6cd618ab3e0c38f5d7e4f44e98fa4ac1dd";
            sha256 = "sha256-+aFvj3qdCsn2wWW8C5QhAwR2hsYqcVBefBsb82J7Unw=";
          };
        } {
          plugins = [
            "android-firmware-lab"
            "voice-orchestrator"
            "jetson-nixos"
            "mobile-nixos-port"
          ];
        })

      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "lkshrk";
            repo = "linear-ai";
            rev = "1c238ce8ed817cf578ea63b9031bbaddd8455717";
            sha256 = "sha256-jsh2sguJ3REPi89IfMqpRYnXloVWyOje5jcs9T/+IUA=";
            rootDir = "skills";
          };
        } {
          plugins = [
            "linear-status"
            "linear-create-issue"
            "linear-refine"
            "linear-implement"
            "linear-close"
            "linear-doctor"
            "linear-nontech-intake"
            "linear-review"
            "linear-repo-reconcile"
          ];
        })
    ];
  };
}

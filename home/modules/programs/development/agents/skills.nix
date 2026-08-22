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
          "ee-workbench" = {
            description = "Work the electronics bench on this desk with the `ee` CLI — bench projects, the parts inventory and its append-only stock ledger, experiments and their measurements, the FreeCAD session behind `ee mechanical`, and `ee git` over the workbench repository. Use for any inventory, experiment or measurement bookkeeping, and read it before committing anything under the workbench data root.";
            tags = ["electronics"];
            content = builtins.readFile ./skills/ee-workbench.md;
          };
        } {
          plugins = ["ee-workbench"];
        })
      (mkInlineSkill {
          "phone" = {
            description = "Drive the Android handsets and emulators and the iOS simulators on this desk with the `phone` CLI — boot a simulator or AVD from cold, pick a device, screenshot or crop it, list the elements on screen, tap, hold, swipe, type, send keys, wait for the screen to catch up, record a clip and cut it into stills, launch or stop an app, open a deep link, reverse a port to a dev server, and run a whole sequence of those against one device. Use for any hands-on mobile device automation from this host.";
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
            rev = "c2e6f80bbc59b676c2a4022ac137b0ac26391539";
            sha256 = "sha256-eFRgxnP7Hd7AhMifVod67B5K/7EXHLr57qnBT+grALs=";
          };
        } {
          plugins = [
            "android-firmware-lab"
            "jetson-nixos"
            "mobile-nixos-port"
            "commit"
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

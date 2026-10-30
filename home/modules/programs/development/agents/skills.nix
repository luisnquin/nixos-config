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
            description = "Find /nix/store paths by name with `nixgrep <search-term>`, covering a package's outputs, its .drv files and companion derivations such as go-modules. Use it instead of `ls /nix/store | grep`. Prints store basenames, one per line, without the /nix/store/ prefix.";
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
          "phone-skill" = {
            description = "Drive Android phones, emulators (AVDs) and iOS simulators, local or on a remote Mac like rose, with the `phone` CLI instead of raw adb, emulator, avdmanager, xcrun simctl or ssh. Boot, pick a device, snapshot the screen, tap, swipe, type, wait, screenshot, record, install, launch, open deep links, reverse ports.";
            tags = ["mobile"];
            content = builtins.readFile ./skills/phone.md;
          };
        } {
          plugins = ["phone-skill"];
        })
      (mkSkill {src = ./skills;} {
        plugins = ["nix-patch" "nix-triage" "squad" "dev-flow"];
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
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "cursor";
            repo = "plugins";
            rev = "ecc249f1e306fc64ddf83c7bed16cacf7c2239db";
            hash = "sha256-okjUS0AVmgoQfCO3nzzqjL1PnZ8X3r8wjtz5SwrFse0=";
          };
        } {
          plugins = [
            "unslop"
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
      (mkSkill {src = pkgs.llm-agents.herdr.src;} {
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
            owner = "ciembor";
            repo = "agent-rules-books";
            rev = "893a88a6fce3a80c565bf39ac65021b43a8b2990";
            sha256 = "sha256-ZdYHWG/4ueNJXjUABAGU/AgzhS56eN5ZH5DU7YLlY3k=";
          };
        } {
          plugins = ["refactoring-guru"];
        })
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "0xc000022070";
            repo = "skills";
            rev = "b516adf493cb3cf87113a61ccea97cc53627fa77";
            sha256 = "sha256-x9X8N+RAbdtoF/HJYQA4ZQX4iG3cVXGv+0yI8n6Mus4=";
          };
        } {
          plugins = [
            "android-firmware-lab"
            "jetson-nixos"
            "mobile-nixos-port"
            "commit"
            "socratic-debate"
            "counter-attack"
            "qa-fleet"
          ];
        })

      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "leonxlnx";
            repo = "taste-skill";
            rev = "6010f91a9935f1c167dfc500fb1a475681892e6b";
            hash = "sha256-25Hc9DiKiDSq+Ae3bTwv9nnk7KziP0cp3Q8srpXE47U=";
          };
        } {
          plugins = ["taste-skill"];
        })

      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "lkshrk";
            repo = "linear-ai";
            rev = "5b94f09eff649ad9125a1d4a57a815fa9fab536d";
            sha256 = "sha256-oSywlXNy7QOxNuvkW0b6QreCgyVnmGv5VNaOGegUPsY=";
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
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "hyperb1iss";
            repo = "hyperskills";
            rev = "a0e30e791c5a9ef65f438a24b6ea7bffb912ac70";
            hash = "sha256-Rl0BBK/Hnf7xOwWvNMqkmm64sRnMEtl0+walbkq79P8=";
            rootDir = "skills";
          };
        } {
          plugins = ["tui-design"];
        })
      (mkSkill {
          # Its install and self-update sections reach for npm -g and curl, which would shadow the nix-built pen.
          src = pkgs.runCommand "pen-design-skill" {} ''
            mkdir -p $out/pen-design
            sed -e '/^### Check installation/,/^### Authentication/{/^### Authentication/!d}' \
                -e '/^### Staying up to date/,/^## Creating a Design/{/^## Creating a Design/!d}' \
                ${pkgs.pen-dev}/lib/node_modules/@pen.dev/cli/SKILL.md > $out/pen-design/SKILL.md
          '';
        } {
          plugins = ["pen-design"];
        })
      (mkSkill {
          src = pkgs.applyPatches {
            name = "ui-ux-pro-max-skill";
            src = pkgs.fetchFromGitHub {
              owner = "nextlevelbuilder";
              repo = "ui-ux-pro-max-skill";
              rev = "d14efff8ed1faa4187f004f79926f428c8074ec7";
              hash = "sha256-oUr1p0GuTH5nXegrf+3Q4qOWUCDOhEEEEpcGaRBvepE=";
              rootDir = ".claude/skills";
            };
            postPatch = ''
              substituteInPlace ui-ux-pro-max/SKILL.md \
                --replace-fail '"''${CLAUDE_PLUGIN_ROOT}/.claude/skills/ui-ux-pro-max/' "\"$out/ui-ux-pro-max/"
            '';
          };
        } {
          plugins = ["ui-ux-pro-max"];
        })
    ];
  };
}

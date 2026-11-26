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
      (vercel-labs.skills {
        plugins = [
          "find-skills"
        ];
      })
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "mattpocock";
            repo = "skills";
            rev = "5863806dd4e15c63174bee4e7515a4256792ecfa";
            hash = "sha256-nxTjN6ePfumLCJ77zw4kdMwPeOHU2TztFZPOiMr3zGw=";
            rootDir = "skills/engineering";
          };
        } {
          plugins = [
            "improve-codebase-architecture"
            "codebase-design"
          ];
        })
      (mkSkill {src = pkgs.llm-agents.herdr.src;} {
        plugins = ["herdr"];
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
            owner = "caldoai";
            repo = "skills";
            rev = "d5b68dace378219890d821adb3685538e2c69b04";
            hash = "sha256-rLNHCZ3Dhwo2lNQcXNsRU6GTsuUgq7bbWhiaNUFzmRc=";
            rootDir = "skills";
          };
        } {
          plugins = ["untangle-codebase"];
        })
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "0xc000022070";
            repo = "skills";
            rev = "95c377d33d72cce933ebbf61d82ddcda1446956b";
            sha256 = "sha256-Jzwkcfv2lwQAYwiR3Mkwo2VsIknRRIjOsvfUIt+ZptI=";
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
            rev = "6c0b5c78f5d31776e2210305abaa210cdc0030bf";
            hash = "sha256-NS6nRA3Leuc8WC6umaLvhyqIRbjTH6w3b88NcDGht4M=";
          };
        } {
          plugins = ["taste-skill"];
        })
    ];
  };
}

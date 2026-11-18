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
            rev = "4ae2317ddb22b046e26455e4f9c9b3aaaf62212d";
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
            rev = "f3cf47774975a7d9fca45730babf4ff7ad765ada";
            hash = "sha256-Q8r94n44dEE4BYGbnJBAl6Qtw/Pq8wSWjCfTGtzdICI=";
            rootDir = "skills";
          };
        } {
          plugins = ["untangle-codebase"];
        })
      (mkSkill {
          src = pkgs.fetchFromGitHub {
            owner = "0xc000022070";
            repo = "skills";
            rev = "f4feaf5020c5e5c159c36e13134dec63481ee1d1";
            sha256 = "sha256-OsqGJyF2AD2KotpzXvqnlBsmyfZd40UrqyIPfkAbrK4=";
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
            rev = "9b4bf937a1f69348126959c15d2f99a73e65699b";
            hash = "sha256-BASbXznKYEcCUOEd0rm+ajcaGNhZ3wjONJP6tkpGEww=";
          };
        } {
          plugins = ["taste-skill"];
        })
    ];
  };
}

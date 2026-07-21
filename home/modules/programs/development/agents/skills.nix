{
  inputs,
  lib,
  pkgs,
  ...
}: let
  inherit (inputs.agentic-flake.lib) mkSkill mkInlineSkill;

  simdeckUpstream = builtins.readFile (pkgs.fetchurl {
    name = "simdeck-skill.md";
    url = "https://raw.githubusercontent.com/NativeScript/SimDeck/8baf1037ece3b37a90bb3040335a9a1283c0b2b5/skills/simdeck/SKILL.md";
    hash = "sha256-VY1IfDIYhGLymhMMhvC8qG2EeNevqGUwdzXbyKqjWHQ=";
  });

  simdeckTopology = ''
    ## Host topology

    Simulators, emulators and the SimDeck service run on `rose`, a Mac reached
    over Tailscale, never on this host. This host ships no SimDeck binary;
    `simdeck` here is a wrapper that ssh's every invocation to `rose`.

    - Paths under `$HOME` are rewritten to rose's `$HOME`. Paths outside it are
      sent verbatim and will usually not exist there.
    - The working directory is translated too, so project-scoped state such as
      `simdeck use <UDID>` still keys off the repository you are in.
    - Artifacts -- screenshots, recordings, `--artifacts-dir` -- are written on
      rose. Prefer `screenshot --stdout > local.png`, which streams back
      byte-exact; otherwise `scp rose:<path> .` before reading them.
    - Builds must be produced on rose for `simdeck install` to resolve them.
    - No TTY is allocated. Never run `simdeck --open`; it would open a browser on
      rose. Point the user at http://rose:4310 instead.
  '';
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
      (wshobson.agents {
        plugins = [
          "typescript-advanced-types"
          "e2e-testing-patterns"
        ];
      })
      (mkInlineSkill {
          "simdeck" = {
            description = "Drive iOS simulators and Android emulators on the remote mac rose — lifecycle, app install/launch, live viewing, UI inspection, touch/keyboard automation, screenshots, recordings, logs, pasteboard, hardware controls, and repeatable flows.";
            tags = ["mobile"];
            content = let
              frontmatterFence = "\n---\n";

              dropFrontmatter = text:
                lib.concatStringsSep frontmatterFence
                (lib.drop 1 (lib.splitString frontmatterFence text));
            in
              simdeckTopology + dropFrontmatter simdeckUpstream;
          };
        } {
          plugins = ["simdeck"];
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
            rev = "96d41d56ba9128a5f51fb4e0d05f2ff86c536ce4";
            sha256 = "sha256-M/MVKuU7p/vgVJuzgPfczZQvRZynu8RQwtsVfyeO7So=";
          };
        } {
          plugins = ["android-firmware-lab"];
        })
    ];
  };
}

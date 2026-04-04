{lib, ...}: let
  mkSources = sourcesList:
    builtins.listToAttrs (map (
        src: let
          parts = lib.splitString "/" src.repo;
          owner = builtins.elemAt parts 0;
          repo = builtins.elemAt parts 1;

          key = lib.strings.sanitizeDerivationName src.repo;
          subdir = src.subdir or "skills";
        in {
          name = key;
          value = {
            path = builtins.fetchTarball {
              url = "https://github.com/${owner}/${repo}/archive/refs/heads/main.tar.gz";
              inherit (src) sha256;
            };
            inherit subdir;
          };
        }
      )
      sourcesList);
in {
  programs.agent-skills = {
    enable = true;
    skills.enableAll = true;

    sources = mkSources [
      {
        repo = "vercel-labs/skills";
        sha256 = "0qpzv67hc38aqh5ilyhq6qd8fvs051v241n12695lykbwy0iykd1";
      }
    ];

    targets = {
      agents.enable = true;
      claude.enable = false;
      antigravity.enable = true;
      cursor.enable = true;
    };
  };
}

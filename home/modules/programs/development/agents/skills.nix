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
        sha256 = "1japp0hia9wyssicadfnd70x1fj3x1w0cxmv41wbakkg8mgfzfqb";
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

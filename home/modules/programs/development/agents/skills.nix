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
        sha256 = "19gq3iafad74vpv0i8rxxhwbj0ysbn0p4nb6w0hxmq7v7229r8d4";
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

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
        sha256 = "0mlnsjg720npn4mfh4457liqblmw1qf484n4glnp1yzaip8g12y3";
      }
    ];

    targets = {
      agents.enable = true;
      claude.enable = true;
      antigravity.enable = true;
      cursor.enable = true;
    };
  };
}

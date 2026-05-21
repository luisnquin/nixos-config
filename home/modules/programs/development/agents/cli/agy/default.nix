{
  pkgs,
  lib,
  ...
}: {
  home.packages = [
    (lib.lowPrio pkgs.llm-agents.antigravity)
  ];
}

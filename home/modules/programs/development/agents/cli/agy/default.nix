{
  pkgs,
  lib,
  ...
}: {
  programs.agy = {
    enable = true;
    package = lib.lowPrio pkgs.llm-agents.antigravity-cli;
  };
}

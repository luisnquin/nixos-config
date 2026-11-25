{pkgs}: let
  b = "(^|[;&|({[:space:]])";

  shared = [
    {
      pattern = "${b}npx[[:space:]]+convex([[:space:]]|$)";
      reason = "Do not call npx convex. Use the package.json scripts (npm run convex:dev, convex:deploy, convex:codegen); read package.json for the rest.";
    }
    {
      pattern = "${b}(pkill|killall)([[:space:]]|$)|${b}kill[[:space:]][^;&|]*\\$\\((pgrep|pidof)|${b}(pgrep|pidof)[^;&]*\\|[[:space:]]*xargs[^;&|]*[[:space:]]kill([[:space:]]|$)";
      reason = "Killing by name or pattern also hits the user's own processes (a pkill -f once killed their `just watch` pane). Kill only PIDs you started (keep $! or a pid file), or run pgrep -af <pattern>, read the list, then kill <pid>.";
    }
    {
      pattern = "${b}herdr[^;&|]*[[:space:]]server[[:space:]]+stop|${b}systemctl[[:space:]]+--user[[:space:]]+(stop|restart|kill)[^;&|]*herdr-hub|${b}tmux[^;&|]*[[:space:]]kill-server";
      reason = "This stops the user's herdr hub or tmux server and every agent pane in it. Ask the user to do it.";
    }
  ];

  polling = {
    pattern = "${b}(until|while)[[:space:]].*sleep[[:space:]]|for[[:space:]].*\\$\\(seq[[:space:]].*sleep[[:space:]]";
    foreground = true;
    reason = "Foreground polling loop refused: it shows no progress until it ends, so the user kills it. Run it with run_in_background: true (you are notified when it exits) or use the Monitor tool. For devices, use `phone wait --timeout`.";
  };

  mk = harness: rules:
    pkgs.writeShellApplication {
      name = "agent-command-rules-${harness}";
      runtimeInputs = [pkgs.jq];
      runtimeEnv.AGENT_COMMAND_RULES = pkgs.writeText "agent-command-rules-${harness}.json" (builtins.toJSON rules);
      text = builtins.readFile ./command-rules.sh;
    };
in {
  claude = mk "claude" (shared ++ [polling]);
  codex = mk "codex" shared;
}

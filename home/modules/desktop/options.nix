{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption types;
  cfg = config.services.github-monitor;
  cacheDir = "${config.xdg.cacheHome}/github-monitor";
  cacheFile = "${cacheDir}/state.json";
  monitorConfig = pkgs.writeText "github-monitor.json" (builtins.toJSON {
    inherit (cfg) workflows issueRepositories;
  });

  update = pkgs.writeShellApplication {
    name = "github-monitor-update";
    runtimeInputs = with pkgs; [coreutils gh jq];
    text = ''
      cache_dir=${lib.escapeShellArg cacheDir}
      cache_file=${lib.escapeShellArg cacheFile}
      config_file=${lib.escapeShellArg monitorConfig}
      login_file="$cache_dir/login"
      mkdir -p "$cache_dir"

      login=""
      if [ -s "$login_file" ]; then
        login="$(<"$login_file")"
      elif login="$(gh api user --jq .login 2>/dev/null)" && [ -n "$login" ]; then
        printf '%s\n' "$login" > "$login_file"
      fi

      work_file="$(mktemp)"
      issue_file="$(mktemp)"
      error_file="$(mktemp)"
      trap 'rm -f "$work_file" "$issue_file" "$error_file"' EXIT
      printf '[]' > "$work_file"
      printf '[]' > "$issue_file"
      printf '[]' > "$error_file"

      while IFS=$'\t' read -r repo workflow; do
        [ -n "$repo" ] || continue
        if result="$(gh run list --repo "$repo" --workflow "$workflow" --limit 1 \
          --json conclusion,databaseId,displayTitle,status,updatedAt,url,workflowName 2>/dev/null)"; then
          jq --arg repo "$repo" --arg workflow "$workflow" \
            '. + [($runs[]? | . + {repo:$repo, configuredWorkflow:$workflow})]' \
            --argjson runs "$result" "$work_file" > "$work_file.next"
          mv "$work_file.next" "$work_file"
        else
          jq --arg source "$repo · $workflow" '. + [$source]' "$error_file" > "$error_file.next"
          mv "$error_file.next" "$error_file"
        fi
      done < <(jq -r '.workflows[] | [.repo, .workflow] | @tsv' "$config_file")

      while IFS= read -r repo; do
        [ -n "$repo" ] || continue
        if result="$(gh issue list --repo "$repo" --state open --limit 100 \
          --json assignees,number,title,updatedAt,url 2>/dev/null)"; then
          jq --arg repo "$repo" --arg login "$login" \
            '. + [($issues[]? | select((.assignees | length) == 0 or any(.assignees[]?; .login == $login)) | . + {repo:$repo})]' \
            --argjson issues "$result" "$issue_file" > "$issue_file.next"
          mv "$issue_file.next" "$issue_file"
        else
          jq --arg source "$repo · issues" '. + [$source]' "$error_file" > "$error_file.next"
          mv "$error_file.next" "$error_file"
        fi
      done < <(jq -r '.issueRepositories[]' "$config_file")

      tmp="$(mktemp "$cache_dir/state.XXXXXX")"
      jq -cn \
        --slurpfile workflows "$work_file" \
        --slurpfile issues "$issue_file" \
        --slurpfile errors "$error_file" \
        --arg updated_at "$(date --iso-8601=seconds)" \
        '{workflows:($workflows[0] | sort_by(.updatedAt) | reverse),
          issues:($issues[0] | sort_by(.updatedAt) | reverse | .[0:3]),
          errors:$errors[0], updated_at:$updated_at,
          workflow_count:($workflows[0] | length),
          failed_count:($workflows[0] | map(select(.conclusion | IN("failure", "cancelled", "timed_out", "startup_failure", "action_required"))) | length),
          issue_count:($issues[0] | length)}' > "$tmp"
      mv "$tmp" "$cache_file"
    '';
  };
in {
  options.services.github-monitor = {
    enable = mkEnableOption "GitHub workflow and issue monitor";

    workflows = mkOption {
      type = types.listOf (types.submodule {
        options = {
          repo = mkOption {type = types.str;};
          workflow = mkOption {type = types.str;};
        };
      });
      default = [];
    };

    issueRepositories = mkOption {
      type = types.listOf types.str;
      default = [];
    };
  };

  config = mkIf cfg.enable {
    systemd.user.services.github-monitor = {
      Unit.Description = "Refresh GitHub workflow and issue status";
      Service = {
        Type = "oneshot";
        ExecStart = lib.getExe update;
      };
    };

    systemd.user.timers.github-monitor = {
      Unit.Description = "Refresh GitHub monitor every 30 seconds";
      Timer = {
        OnBootSec = "5s";
        OnUnitActiveSec = "30s";
        Unit = "github-monitor.service";
      };
      Install.WantedBy = ["timers.target"];
    };
  };
}

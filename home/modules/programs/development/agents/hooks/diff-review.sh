# Claude PostToolUse hook: once the uncommitted diff crosses a size level,
# ask the agent to self-review it. Each level fires once per HEAD.

case "${AGENT_DIFF_REVIEW:-}" in
off | 0 | no | skip) exit 0 ;;
esac

read -ra levels <<<"${AGENT_DIFF_REVIEW_LEVELS:-300 800 2000}"

file=$(jq -r '.tool_input.file_path // .tool_input.notebook_path // empty')
[[ -n $file ]] || exit 0
dir=$(dirname -- "$file")
[[ -d $dir ]] || exit 0
root=$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null) || exit 0

head=$(git -C "$root" rev-parse -q --verify HEAD) || head=4b825dc642cb6eb9a060e54bf8d69288fbee4904

excludes=(':!*.lock' ':!*-lock.json' ':!*-lock.yaml' ':!*.snap' ':!*.svg' ':!*.min.*')

tracked=$(git -C "$root" diff --numstat "$head" -- . "${excludes[@]}" |
  awk '$1 != "-" { n += $1 + $2 } END { print n + 0 }')

untracked=0
while IFS= read -r -d '' f; do
  path="$root/$f"
  [[ -f $path && $(stat -c %s -- "$path") -le 1048576 ]] || continue
  grep -Iq . -- "$path" || continue
  untracked=$((untracked + $(wc -l <"$path")))
done < <(git -C "$root" ls-files -z --others --exclude-standard -- . "${excludes[@]}")

total=$((tracked + untracked))

state_dir="${XDG_STATE_HOME:-$HOME/.local/state}/agent-diff-review"
state="$state_dir/$(printf '%s' "$root" | sha1sum | cut -c1-16)"
fired=0
if [[ -r $state ]]; then
  read -r seen_head seen_level <"$state" || true
  [[ ${seen_head:-} == "$head" ]] && fired=${seen_level:-0}
fi

level=0
for l in "${levels[@]}"; do
  ((total >= l)) && level=$l
done
((level > fired)) || exit 0

mkdir -p -- "$state_dir"
printf '%s %s\n' "$head" "$level" >"$state"

msg="The uncommitted diff in $root is now $total lines (review level $level). Before continuing, review the whole change since HEAD, not only the last edit, for: logic duplicated where an existing helper already covers it; abstractions with a single implementation or a single caller; functions whose parameter lists keep growing; state shared across modules that could be passed explicitly. Fix only what is clearly wrong and do not add structure speculatively. To diagnose a smell and pick its treatment, load the refactoring-guru skill. If nothing needs fixing, say so in one line and continue."

jq -n --arg msg "$msg" '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $msg}}'

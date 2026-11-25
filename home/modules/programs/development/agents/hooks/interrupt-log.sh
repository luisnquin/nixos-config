input=$(cat)
transcript=$(jq -r '.transcript_path // empty' <<<"$input")
[[ -r $transcript ]] || exit 0
recent=$(tail -n 40 "$transcript")
grep -q 'Request interrupted by user' <<<"$recent" || exit 0

log=${XDG_STATE_HOME:-${HOME%/}/.local/state}/agent-interrupts.jsonl
jq -cn --argjson in "$input" --arg pane "${HERDR_PANE_ID:-${TMUX_PANE:-}}" '
  def ts: sub("\\.[0-9]+Z$"; "Z") | fromdate;
  [inputs | select(.type == "user" or .type == "assistant")] as $e
  | ($e | map(.type) | rindex("user")) as $i
  | select($i != null and ($e[$i] | tostring | test("Request interrupted by user")))
  | ([$e[:$i][] | select(.type == "assistant")] | last) as $p
  | {
      ts: $e[$i].timestamp,
      session: $in.session_id,
      cwd: $in.cwd,
      pane: $pane,
      during: (if $e[$i] | tostring | test("for tool use") then "tool" else "text" end),
      last: ($p.message.content // [] | map(select(.type == "tool_use") | .name + " " + ((.input.command // .input.file_path // .input.description // "") | tostring | .[0:200])) | join("; ")),
      ran_s: (if $p then ($e[$i].timestamp | ts) - ($p.timestamp | ts) else null end),
      next: ($in.prompt // "" | .[0:300])
    }' <<<"$recent" >>"$log"

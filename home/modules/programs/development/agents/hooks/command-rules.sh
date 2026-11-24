input=$(cat)
cmd=$(jq -r '.tool_input.command // empty' <<<"$input")
[[ -n $cmd ]] || exit 0
bg=$(jq -r '.tool_input.run_in_background // false' <<<"$input")

while IFS= read -r -d '' pattern && read -r -d '' foreground && read -r -d '' reason; do
  [[ $foreground == true && $bg == true ]] && continue
  if [[ $cmd =~ $pattern ]]; then
    jq -n --arg r "$reason" '{hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "deny",
      permissionDecisionReason: $r
    }}'
    exit 0
  fi
done < <(jq -j '.[] | .pattern, "\u0000", (.foreground // false | tostring), "\u0000", .reason, "\u0000"' "$AGENT_COMMAND_RULES")

# claude's builtin check prompts on rm with a runtime-resolved target even in bypass
# mode, stalling unattended sessions; denying makes the agent retry with a literal path

cmd=$(jq -r '.tool_input.command // empty')
[[ -n $cmd ]] || exit 0

grep -qE '(^|[;&|(]|\bsudo)[[:space:]]*rm[[:space:]][^;&|]*(\$|`)' <<<"$cmd" || exit 0

jq -n '{hookSpecificOutput: {
  hookEventName: "PreToolUse",
  permissionDecision: "deny",
  permissionDecisionReason: "rm target comes from $(...)/$var/backticks, which forces a blocking permission prompt. Retry with the literal path written out (e.g. rm -rf /tmp/foo-test)."
}}'

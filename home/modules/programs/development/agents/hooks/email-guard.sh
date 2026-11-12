# claude leaks its injected account email into tool calls (anthropics/claude-code#107757)

email=$(jq -r '.oauthAccount.emailAddress // empty' "${CLAUDE_CONFIG_DIR:-$HOME}/.claude.json" 2>/dev/null)
[[ -n $email ]] || exit 0

jq -c '.tool_input // empty' | grep -qiF -e "$email" -e "${email/@/%40}" || exit 0

jq -n '{hookSpecificOutput: {
  hookEventName: "PreToolUse",
  permissionDecision: "ask",
  permissionDecisionReason: "tool input contains the account email address"
}}'

# herdr-recall zsh hook: record the command a pane runs on preexec, and its exit
# status on precmd. Only active inside a herdr pane (HERDR_PANE_ID is set by herdr).

_herdr_recall_bin="${${(%):-%N}:A:h:h}/bin/herdr-recall"

if [[ -n ${HERDR_PANE_ID:-} && -x $_herdr_recall_bin ]]; then
  _herdr_recall_preexec() {
    # $2 is the alias-expanded command line; $1 is what was typed.
    ("$_herdr_recall_bin" preexec "${2:-$1}" &) >/dev/null 2>&1
  }
  _herdr_recall_precmd() {
    local code=$?
    ("$_herdr_recall_bin" precmd "$code" "$PWD" &) >/dev/null 2>&1
  }
  autoload -Uz add-zsh-hook
  add-zsh-hook preexec _herdr_recall_preexec
  # Prepended rather than added: $? in a precmd function is the status of
  # whatever ran before it, so any earlier hook would mask the real exit code.
  precmd_functions=(_herdr_recall_precmd ${precmd_functions:#_herdr_recall_precmd})
fi

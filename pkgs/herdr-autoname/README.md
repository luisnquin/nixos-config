# herdr-autoname

tmux-like automatic tab names for [herdr](https://github.com/ogulcancelik/herdr).

A tab is named after, in order: the coding agent herdr detected in its active
pane, the pane's foreground program, or the working directory basename.

## Behaviour

- Renaming a tab yourself opts it out. The plugin remembers the last name it set
  per tab; a label that is neither that name nor herdr's own tab number is left
  alone, and `tab.renamed` clears the entry. Rename the tab back to its number to
  opt in again.
- Event hooks rename only the tab the event carries; the full sweep runs on the
  startup hook and the `refresh` action.
- Unfocused multi-pane tabs keep the name they already have — there is no
  per-tab active pane in the API, only global focus.
- Wrappers are skipped: `sudo nvim`, `env RUST_LOG=debug nvim` and
  `timeout 30 nvim` all name the tab `nvim`. Options that take a value are not
  parsed, so `sudo -u other nvim` names the tab `other`.

## Config

`$XDG_CONFIG_HOME/herdr-autoname/config`, `key = value` per line. The path is
fixed rather than `HERDR_PLUGIN_CONFIG_DIR` because the shell hook runs outside
the plugin environment and has to read the same file.

| key | default | meaning |
| --- | --- | --- |
| `idle` | `cwd` | name for a shell at a prompt: `cwd` basename or `shell` |
| `agent` | `true` | prefer herdr's detected agent over the foreground program |
| `max_len` | `20` | truncate names to this many characters |

`HERDR_AUTONAME_IDLE`, `HERDR_AUTONAME_AGENT` and `HERDR_AUTONAME_MAX_LEN`
override the file.

State lives in `$XDG_STATE_HOME/herdr-autoname/tabs`, one file per tab.

## Shell hook

herdr has no "foreground command changed" event, so per-command names come from
`shell/hook.zsh` (sourced from the installed plugin directory). Only zsh is
covered.

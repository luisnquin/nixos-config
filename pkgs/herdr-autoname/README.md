# herdr-autoname

tmux-like automatic tab and space names for
[herdr](https://github.com/ogulcancelik/herdr).

A tab is named after, in order: the coding agent herdr detected in its active
pane, the pane's foreground program, or the working directory basename.

A space is named after the repository holding the pane of its *active* tab, or
that pane's directory basename. herdr derives its own space label from the first
tab's root pane instead, so a background pane decides the name of the space you
are looking at, and the label stops matching what is on screen.

Panes running an agent and spaces also get a Nerd Font glyph, reported as a
metadata token rather than folded into the name — a name doubles as the opt-out
marker, so a glyph inside it would make every rename you type look like ours.

## Behaviour

- Renaming a tab yourself opts it out. The plugin remembers the last name it set
  per tab; a label that is neither that name nor herdr's own tab number is left
  alone, and `tab.renamed` clears the entry. Rename the tab back to its number to
  opt in again.
- Renaming a space yourself opts it out too, but the marker is persistent: a
  space label is always cwd-shaped, so herdr's own label cannot be told apart
  from one you typed. An untouched space is adopted once; afterwards only the
  plugin's own label is replaced, and a foreign label seen on `workspace.renamed`
  writes an opt-out marker. Rename the space back to the automatic name to opt in
  again.
- The pane a space is named after is the focused pane of its active tab, else
  that tab's first pane — a stale space name beats a missing one.
- `cwd` is preferred over `foreground_cwd` for spaces, so a child process that
  chdirs elsewhere does not drag the whole space along with it.
- Event hooks rename only the tab and space the event carries; the full sweep
  runs on the startup hook and the `refresh` action.
- Icons are reported even for a space you opted out of naming: they are a
  separate display token, not part of the label.
- A space glyph follows the project kind of the repository holding its pane
  (`flake.nix`, `Cargo.toml`, `go.mod`, `package.json`, `pyproject.toml`,
  `requirements.txt`), else the repository, else a plain directory. A pane glyph
  follows the detected agent.
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
| `icons` | `true` | report `$icon` metadata tokens for agent panes and spaces |
| `max_len` | `20` | truncate names to this many characters |
| `space` | `repo` | space name source: `repo` root basename, `cwd` basename, or `off` |
| `icon.<key>` | — | replace one glyph; `<key>` is an agent name (`claude`, `codex`, …) or a project kind (`nix`, `rust`, `go`, `node`, `python`, `git`, `dir`) |

An `icon.<key>` value is capped at two characters — a sidebar row lays its tokens
out by display width, so a longer one shifts every row it lands in. An empty
value restores the built-in glyph.

`HERDR_AUTONAME_IDLE`, `HERDR_AUTONAME_AGENT`, `HERDR_AUTONAME_ICONS`,
`HERDR_AUTONAME_MAX_LEN` and `HERDR_AUTONAME_SPACE` override the file.

Rendering the glyphs needs the token in herdr's own config, since the sidebar
rows are opt-in:

```toml
[ui.sidebar.agents]
rows = [["state_icon", "$icon", "state_text"], ["terminal_title_stripped"]]
```

State lives in `$XDG_STATE_HOME/herdr-autoname`, one file per tab under `tabs/`,
per space under `spaces/` and per reported glyph under `icons/`; a `<id>.off`
file next to a space entry is its opt-out marker.

## Shell hook

herdr has no "foreground command changed" event, so per-command names come from
`shell/hook.zsh` (sourced from the installed plugin directory). Only zsh is
covered.

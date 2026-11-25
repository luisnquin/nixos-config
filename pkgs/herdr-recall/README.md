# herdr-recall

Remembers what every [herdr](https://github.com/ogulcancelik/herdr) pane was
running, so restarting the server does not cost you the thread.

A restart restores panes and their directories, but not what was in them. For an
agent pane herdr can do better — it persists the agent session id and replays
`claude --resume <uuid>` on restore — but only for sessions an agent integration
reported. Everything else, and every plain shell, is gone.

`herdr-recall show` prints one line per pane: the resume invocation for an agent,
the last command line for a shell.

```
sample-app
  w2F:p4     ~/Projects/github.com/acme/sample-app
    codex resume 019a3f7c-2b41-7a10-9c55-2f0a1d8e4b33

.dotfiles (w2K)
  w2K:p1     ~/.dotfiles
    nix build .#nixosConfigurations.nyx.config.system.build.toplevel  (exit 1)
```

Pane ids survive a restart because herdr persists workspace ids and public pane
numbers, so the sheet still lines up with the panes you are looking at. A leading
`-` marks a pane herdr no longer has.

## Behaviour

- Agent panes are swept from the socket API on every pane and space event, and
  once on startup. The sweep records the agent and, when one was reported, the
  `agent_session` herdr itself resumes from.
- A resume line is only written for a session herdr vouched for: the source has
  to be `herdr:<agent>`, matching `agent_resume::plan` upstream. A session
  reported by a plugin under its own source is recorded but not turned into a
  command.
- An agent pane whose session was never reported still gets a line, marked
  `(no session recorded)` — knowing the pane held claude beats dropping it for
  want of an id. That marker means the agent's herdr integration is not
  installed, or its hook never fired.
- Agent wins over shell: a pane where you ran a shell and then launched claude
  comes back as claude.
- Shell commands come from `shell/hook.zsh`, because herdr has no "foreground
  command changed" event. Only zsh is covered.
- The finished command is reported back as a `$last` metadata token, so a sidebar
  row can show it. Nothing is reported while the command is still running — the
  tab name already tracks that, and for agent panes nothing is reported at all.
- Entries for panes herdr no longer knows about are kept for 7 days, then swept
  on the next event. That is what makes the sheet useful after a crash.

## Commands

| command | meaning |
| --- | --- |
| `herdr-recall show` | print the sheet |
| `herdr-recall show --json` | the same entries, unformatted |
| `herdr-recall sync` | refresh from the socket API; also the event hook |
| `herdr-recall mine` | start the agents herdr restored but left typed, newest first |
| `herdr-recall park` | stop every running agent, leave its resume line typed, then open `mine` |

`mine` is the welcome screen the hub terminal opens once per boot. With
`session.resume_agents_on_restore`, herdr types each agent's resume line into
its pane but does not run it. The screen lists those spaces the way the sidebar
does, three to a colored group, with each conversation's title. Space marks
groups; Enter starts the marked ones, or the one under the cursor when none is
marked, with no prompt sent, and focuses the first in herdr. The rest keep
their line typed. Esc starts nothing. A pane whose prompt no longer shows its
resume line is left alone.

`park` (prefix+p in herdr) is the way back for a context switch. It first
lists the running agents. Esc cancels and touches nothing; Enter Ctrl+Cs
each agent herdr can resume, reports its session again since herdr forgets it
on exit, and types the resume line, so the panes look as they do after a boot,
then opens `mine`. An agent that will not exit is left running.

`preexec` and `precmd` are for the shell hook.

State lives in `$XDG_STATE_HOME/herdr-recall/panes`, one JSON file per pane, so
concurrent writers never clobber each other.

## Sidebar

Rendering `$last` needs the token in herdr's own config, since the sidebar rows
are opt-in:

```toml
[ui.sidebar.agents]
rows = [["state_icon", "state_text"], ["terminal_title_stripped"], ["$last"]]
```

## Shell hook

`shell/hook.zsh` registers `preexec` and `precmd` from the installed plugin
directory. It prepends its `precmd` rather than appending: `$?` inside a precmd
function is the status of whatever ran before it, so an earlier hook would mask
the real exit code.

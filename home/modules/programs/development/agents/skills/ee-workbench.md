# ee-workbench

`ee` is the electronics workbench on this desk: bench projects, a parts
inventory, experiments and measurements, kept as plain TOML in one Git
repository under `$XDG_DATA_HOME/ee-workbench` (or `$EE_WORKBENCH_DATA`).

**The CLI documents itself. `ee --help` carries the command tree, `ee <domain>
<verb> --help` carries the flags.** This file covers what the CLI cannot tell
you: how the storage behaves and when you are allowed to commit.

Bare `ee` opens a TUI — that is the human surface, it will hang an agent. Always
call a subcommand. Every query takes `--json`, and that JSON is the on-disk
record verbatim; parse it instead of the table output.

## The repository is the database

There is no server, no sync, no cache to invalidate. `ee` reads and writes files
and nothing else, so `ee git` shows exactly what changed and a reviewer can read
it.

Two directories are **append-only**: `inventory/events/` and `measurements/`.
Each record is written once under a unique `<timestamp>-<hex>` id and never
edited afterwards. On-hand stock is summed from the deltas, never stored — a
wrong `receive` is corrected with an opposing `consume`, not by touching the
file. `projects/`, `inventory/parts/` and `experiments/` do hold mutable
records, but only through their own verbs.

Never hand-edit files under the data root, and never `ee git rm` a ledger event.
`ee repo check` parses everything and reports dangling references and negative
stock; run it after any batch and before committing.

Project identity is the slug plus the generated id inside the record. Absolute
paths belong to no one else's machine, so a checkout is mapped with
`ee project link <project> <path>`, which writes to XDG state, outside the
repository.

## Committing

`ee` never commits, pulls or pushes on its own. You commit, through
`ee git <args...>` — plain git, run in the workbench repository.

Commit **after one coherent domain mutation, or one coherent batch of them**,
and only once you have:

1. seen the command exit 0,
2. read the generated change (`ee git status --short`, `ee git diff`), and
3. confirmed it contains only what you just did.

Do **not** commit:

- after reads — a query changes nothing,
- after a failed or half-finished operation; fix or undo it first,
- when the tree already carries unrelated changes you did not make. Leave them
  and say so; do not fold someone else's bench notes into your commit.

Stage the specific paths the mutation produced, never `ee git add -A`. Never
`ee git push` and never `ee git pull` — the human decides when this repository
meets any other copy of itself.

## Mechanical

`ee mechanical` drives a real FreeCAD session. The session is
`ee-freecad-server`, a native binary that links the installed FreeCAD and owns
`$XDG_RUNTIME_DIR/ee-workbench/cad.sock`; `ee` is only the client. **Start
nothing yourself** — the first verb that needs a session starts one and waits
for it, and an idle session retires after 15 minutes. `ee mechanical status` is
the one verb that never starts anything, so it stays a safe probe.

`ee` is installed together with the exact server it was built against, so the
two cannot drift. If a session started by an older generation is still holding
the socket, every verb but `status`, `document save` and `session stop` refuses
and says so: save what is open, stop the session, and the next command starts
the right one.

A session with a document nobody saved refuses to retire, and
`ee mechanical session stop` refuses too unless you pass `--force`. Save before
you stop, and treat a `--force` stop as discarding work.

```sh
ee mechanical document new --name Plate --json
ee mechanical body new --name Bar --json
ee mechanical sketch new --plane xy --json
ee mechanical sketch rectangle --width 40 --height 20 --centered \
  --name-width bar_x --name-height bar_y --json
ee mechanical pad new --length 6 --json
ee mechanical sketch new --plane xy --offset-z 6 --json
ee mechanical sketch circle --radius 4 --x 12 --json
ee mechanical pocket new --through-all --json
ee mechanical document inspect --json
ee mechanical document save --path ~/cad/plate.FCStd --json
```

Every `--path` is resolved where you type it, so relative paths and `~` mean
what they do in your shell. The session is a daemon started from some other
directory and outlives it; it never guesses at a path of its own.

Reopening renames: FreeCAD takes a document's internal name from the file, so
`plate.FCStd` comes back as `plate` whatever `document new --name` called it.
Read the `document` every response echoes rather than assuming the old name.

### Placement

A sketch is not welded to the global origin. `sketch new` takes
`--offset-x/--offset-y/--offset-z` along the plane's own axes and `--rotate`
about its normal, and reports the resulting `basis` — origin, x, y and normal in
global millimetres — so you can check where you actually are before drawing.
Geometry takes `--x/--y` in sketch coordinates, and `sketch rectangle
--centered` reads them as the centre instead of the lower left corner. Centred
means centred: it is held by a symmetry constraint, so it survives a later
change of width.

### Named dimensions

`--name-width`, `--name-height` and `--name-radius` name the constraint that
drives a dimension. `ee mechanical param list --json` reads them back and
`ee mechanical param set <name> <value>` drives them; follow with
`document recompute`. The names are FreeCAD constraint names, so they survive a
save and reopen. Anything unnamed is still a number you have to redraw to
change.

### Seeing what you built

`dof: 0` proves the sketch is determined, not that it is right. Two verbs
answer that:

- `document inspect --json` reports, per solid, the bounding box (`min`, `max`,
  `size`, `centre`), `volume` and `centre_of_mass`, plus one overall `bbox`.
  Numbers are millimetres rounded to a micron. Check them; a wrong model is
  usually wrong in the bounding box first.
- `preview render --path x.png --view iso|front|top|...` rasterizes the model
  offscreen to a PNG you can open. Silhouettes, creases and pocket rims are
  outlined, and a red/green/blue triad marks x/y/z.

`preview export` is the other output: a printable STL that keeps following the
model until the session ends.

### The vocabulary

Sketches: `rectangle`, `circle`. Solids: `pad` (add), `pocket` (remove), both
with `--midplane`, `--reversed` and a `length` you can retarget afterwards
(`pad length`, `pocket length`); `pocket` also takes `--through-all`. That is
all of it. One sketch holds one primitive, so a part is several sketches:
`--sketch` defaults to the newest one and every response echoes which sketch it
used, so read it back rather than assuming. `--body` and `--document` are
stricter — with more than one they refuse rather than guess.

Documents are not workbench records: nothing here touches the ledger, and a
saved `.FCStd` is only tracked if the human puts it in the repository.

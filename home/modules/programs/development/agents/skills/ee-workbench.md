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
ee mechanical sketch rectangle --width 40 --height 20 --centered --json
ee mechanical pad new --length 6 --json

ee mechanical param new bar_x 40 --json
ee mechanical sketch set width bar_x --json
ee mechanical param new hole_x "=bar_x / 2 - 8" --json

ee mechanical sketch new --plane xy --offset-z 6 --json
ee mechanical sketch circle --radius 4 --x hole_x --json
ee mechanical pocket new --through-all --json

ee mechanical param set bar_x 70 --json
ee mechanical document inspect --features --json
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

### Parameters

Every numeric argument takes a number **or the name of a parameter**:
`--width 40` and `--width bar_x` are the same argument in its two forms. That
holds for sketch dimensions, `--x/--y`, the placement offsets, `--rotate` and
`pad`/`pocket` lengths.

Which form a slot holds is an edit, not a property of how it was created:

```sh
ee mechanical param new bar_x 40 --json          # declare
ee mechanical sketch set width bar_x --json      # bind an existing dimension
ee mechanical sketch set width 33 --unbind --json  # back to a literal
ee mechanical pad length thick --json            # the same for a feature
```

A literal over a slot a parameter drives is refused; `--unbind` is how you mean
it. You never have to destroy a parameter other slots still use in order to
free one of them.

Parameters are the document's only arithmetic. A slot holds a number or the
name of one parameter and never an expression; the expression lives in the
parameter:

```sh
ee mechanical param new hole_x "=bar_x / 2 - 8" --json
```

So `param list` is a complete description of what can move, not an index of
where to look for it. It reports each parameter's value, its expression, and
every slot that follows it, plus `orphans` — named dimensions nothing drives
yet. Adopting one is two commands: `param new <name> <value>` then
`sketch set <slot> <name> --sketch <sketch>`, and the orphan row prints that
second line already filled in. Name the sketch: without `--sketch` the verb
takes the newest one, which on a document with several is rarely the row you
were reading. There is no migration verb because none is needed.

Every dimension is named when it is drawn (`width`, `height`, `radius`, `x`,
`y`), and a sketch's placement is addressable the same way (`offset_x`,
`offset_y`, `offset_z`, `rotate`). Nothing has to be decided in advance.

`param remove <name>` refuses while slots still follow it; `--force` freezes
each one at its current value and the response names every slot it froze.

**`param set` can break the model.** One parameter driving six slots can push a
pocket outside its material or a pad to zero length, and FreeCAD leaves those
features in error rather than refusing. So `param set` names the features that
did not build and why, and **exits nonzero**; `document inspect --features`
carries the same per-feature error state. Check the exit status. A bounding box
that prints is not a model that built.

**An expression that does not evaluate is refused, and nothing is left behind.**
`param new` that fails to compute removes the parameter it was declaring, and
takes the registry with it when that call created it; a failed `param set`
restores the expression it replaced, not merely the number. So a nonzero exit
from either verb means the document is exactly as it was. The cost is that
forward references are impossible — you cannot write `=head_len / 2` before
`head_len` exists — which for a registry is the right trade: a parameter that
cannot be evaluated cannot drive anything, so nothing is lost by refusing it at
the point it is written.

**One broken expression stops the whole registry.** FreeCAD evaluates the
parameter set in one pass and abandons the rest at the first failure, so a
single bad row leaves other rows showing the number they last computed. They are
not wrong-looking: the value is plausible and the expression beside it is valid.
Every row therefore carries a `state`:

| state | meaning |
| --- | --- |
| `ok` | the value is what this row's expression produces |
| `invalid` | this row's expression does not evaluate; it is the culprit, and `error` carries FreeCAD's diagnostic |
| `not-evaluated` | this row never ran, so its value is not what its expression produces |

`not-evaluated` is not "downstream" — such a row need not reference the broken
one at all. It is a collateral sibling of a recompute that stopped early, and
which rows get labelled depends on evaluation order, so the same breakage marks
different rows depending on their names. Repair every `invalid` row and the rest
clear on the next recompute.

**`param list` exits nonzero while any row is not `ok`**, and names the culprit
in both surfaces. It is the one listing you reach for when something already
looks wrong, so it does not report success on a registry that stopped.

### Taking things back out

`feature remove <name>` is the only verb that makes the model smaller, and so
the only one that has to repair links FreeCAD would rather clear. Removing a
feature from the middle of a body drops the link the feature above it was built
on, and FreeCAD sets that link to nothing rather than to the feature below: the
body then rebuilds to the material under the hole and reports itself up to
date. `feature remove` relinks it, so the tree closes over the gap.

`--dry-run` reports what the removal would change and changes nothing. It is
the same plan the real run applies — the server computes it once and the flag
decides only whether to apply it — so a preview cannot describe an edit
different from the one that follows it.

```sh
ee mechanical feature remove Pad2 --dry-run --json
ee mechanical feature remove Pad2 --json
```

Either way the response is the blast radius: what goes, which link is relinked
and to what, whether the body's tip moves, which sketches are left behind, and
which parameters lose their last slot.

- **The profile stays.** Removing a pad leaves the sketch it consumed in the
  document, where `param list` picks it up as an orphan. Taking that out too is
  a second `feature remove`, not a policy this verb decides for you.
- **Emptying a body is allowed**, and a body whose tip is nothing has no shape
  at all rather than an empty one: it stops appearing among `inspect`'s solids
  and `preview export` refuses. Padding into it again brings it back.
- **Parameters are never deleted on your behalf.** One whose last slot went
  away survives driving nothing and binds straight back when you rebuild.
- Three refusals: a sketch a live feature still draws from (remove that feature
  first), a feature some parameter's expression reads (point the parameter
  elsewhere first — nothing here rewrites arithmetic you wrote), and a body,
  because what removing one means depends on booleans, which do not exist yet.

**There is no undo**, and none is coming until the session question is settled:
an idle session retires and the next verb starts a fresh one, so an undo stack
would be discarded silently at exactly the moment it was wanted. Save before a
removal you are unsure of, and `--dry-run` first — it costs nothing.

### Seeing what you built

`dof: 0` proves the sketch is determined, not that it is right. Two verbs
answer that:

- `document inspect --json` reports, per solid, the bounding box (`min`, `max`,
  `size`, `centre`), `volume` and `centre_of_mass`, plus one overall `bbox`.
  Numbers are millimetres rounded to a micron. Check them; a wrong model is
  usually wrong in the bounding box first. `--features` adds the build order:
  per body, each feature in turn with the sketch it consumed, that sketch's
  plane, offset and dimensions, what drives each of them, and any error.
- `preview render --path x.png --view iso|front|top|...` rasterizes the model
  offscreen to a PNG you can open. Silhouettes, creases and pocket rims are
  outlined, and a red/green/blue triad marks x/y/z.

`preview export` is the other output: a printable STL that keeps following the
model until the session ends.

### The vocabulary

Sketches: `rectangle`, `circle`. Solids: `pad` (add), `pocket` (remove), both
with `--midplane`, `--reversed` and a `length` you can retarget afterwards
(`pad length`, `pocket length`), and `sketch set` for a sketch dimension or
placement; `pocket` also takes `--through-all`. Removal: `feature remove`, for
a pad, a pocket or a sketch nothing draws from. That is all of it. One sketch holds one primitive, so a part is several sketches:
`--sketch` defaults to the newest one and every response echoes which sketch it
used, so read it back rather than assuming. `--body` and `--document` are
stricter — with more than one they refuse rather than guess.

Documents are not workbench records: nothing here touches the ledger, and a
saved `.FCStd` is only tracked if the human puts it in the repository.

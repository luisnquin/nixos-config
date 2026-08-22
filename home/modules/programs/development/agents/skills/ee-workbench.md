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
`$XDG_RUNTIME_DIR/ee-workbench/cad.sock`; `ee` is only the client. Nothing
spawns it — if `ee mechanical status` reports the session is not running, ask
the human to start it rather than starting it yourself, because a session holds
open documents that may be theirs.

```sh
ee mechanical status --json
ee mechanical document new --name Plate --json
ee mechanical body new --json
ee mechanical sketch new --plane xy --json
ee mechanical sketch rectangle --width 40 --height 25 --json
ee mechanical document recompute --json
ee mechanical document save --path ~/cad/plate.FCStd --json
ee mechanical document inspect --json
```

`sketch rectangle` produces a fully constrained rectangle (`dof: 0`) and
`document inspect` reports objects, sketch geometry, constraints and degrees of
freedom, so verify with `inspect` instead of assuming a mutation landed. Sketch
and body arguments may be omitted only when the document holds exactly one;
otherwise name them.

Documents are not workbench records: nothing here touches the ledger, and a
saved `.FCStd` is only tracked if the human puts it in the repository.

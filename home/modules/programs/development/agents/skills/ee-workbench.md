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
`y`, a line's `x1`..`y2`, a polyline's `x1`..`yN`), and a sketch's placement is
addressable the same way (`offset_x`, `offset_y`, `offset_z`, `rotate`).
Nothing has to be decided in advance. A sketch holds many primitives, so the
second one's names arrive suffixed — `radius_2`, `x1_3` — and every drawing
verb's reply carries a `slots` object mapping each canonical name to the one it
actually got; read that back instead of guessing the suffix.

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

**A creating verb that fails to recompute rolls itself back.** `pad`, `pocket`,
`revolve`, `groove`, `mirror`, `pattern linear`/`polar`, `fillet` and `chamfer`
all remove the feature they just added and restore the body's previous tip
before exiting nonzero, with FreeCAD's own diagnostic in the message. This is
the opposite of `param set`: one parameter can drive features all over a
document, so there is no single feature to roll back to and `param set` leaves
the broken ones in place and names them instead. A failed creating verb, by
contrast, only ever broke the one thing it just built, so undoing that one
thing is unambiguous — and it means a failed `fillet new` or `pattern linear`
never leaves a half-built feature for the next command to trip over.

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
  elsewhere first — nothing here rewrites arithmetic you wrote), and a body —
  `feature remove` only ever shrinks one body's own chain, never a body
  itself.

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
  `--tree` is the same listing with two things `--features` doesn't carry: each
  feature's `volume_delta` and `bbox_delta` — what that one feature added or
  removed, not the body's running total — and each sketch's `basis` and
  `primitives` alongside its dimensions. Diffing consecutive `--features`
  replies by hand is the thing `--tree` replaces.
- `preview render --path x.png --view iso|front|top|...` rasterizes the model
  offscreen to a PNG you can open. Silhouettes, creases and pocket rims are
  outlined, and a red/green/blue triad marks x/y/z.

`preview export` is the other output: a printable STL that keeps following the
model until the session ends.

### The vocabulary

Sketches: `rectangle`, `circle`, `line`, `arc`, `polyline`
(`--points "x,y x,y ..."`, `--close` for a closed wire). Every primitive lands
fully constrained — the reported `dof` is the solver's own count — and every
coordinate or dimension is a number or a parameter name like anywhere else.

An arc is endpoints plus radius: counter-clockwise from `--x1/--y1` to
`--x2/--y2`, `--large` for the major arc, swap the endpoints to bulge the other
way. Endpoints-plus-radius over centre-plus-angles because every slot stays a
length a parameter can drive; a radius shorter than half the chord is refused
with the minimum named.

A sketch holds as many primitives as you draw into it. Lines and arcs whose
endpoints meet close one wire, and a closed loop inside another becomes a hole,
so a plate and all its holes are one sketch and one pocket. A pocket cuts away
from the sketch's normal: a holes sketch belongs on the face it enters (the
example's `--offset-z 6`), and one drawn at the bottom of the material removes
nothing and says nothing.

Solids: `pad` (add) and `pocket` (remove) extrude a sketch along its normal;
`revolve` (add) and `groove` (remove) spin one about an axis instead, for
anything round a straight extrude cannot make. All four take `--midplane`,
`--reversed` and a driven dimension you can retarget afterwards (`pad length`,
`pocket length`, `revolve angle`, `groove angle`); `pad` and `pocket` also take
`--taper` for a drafted wall, and `pocket` takes `--through-all`. `sketch set`
covers a sketch dimension or placement. Removal: `feature remove`, for a pad, a
pocket, a revolve, a groove, a mirror, a pattern, a fillet, a chamfer, a loft
or a sketch nothing draws from — it takes the same `--body`/`--document`/
`--json` as everything else, alongside its positional `FEATURE`.
`--sketch` defaults to the newest one and every response echoes which sketch it
used, so read it back rather than assuming. `--body` and `--document` are
stricter — with more than one they refuse rather than guess.

`loft new --sketch <name>... [--ruled] [--closed]` and
`loft pocket --sketch <name>... [--ruled]` build a solid between two or more
sketches instead of extruding one along an axis — the first `--sketch` is the
profile, the rest are sections in the order given, so orientation is the order
you name them in, not a property of the sketches themselves. `--ruled`
connects sections with straight lines instead of a smoothed surface,
`--closed` wraps the last section back to the first. Fewer than two sketches,
a sketch already consumed by another feature, or two named sketches sharing a
plane are all refused before anything is built. `loft new` adds
(`PartDesign::AdditiveLoft`), `loft pocket` cuts (`PartDesign::SubtractiveLoft`).

`revolve`/`groove` take `--axis x` or `--axis y` (default `y`), the sketch's
own in-plane axes — the only axis every profile already has for free, without
naming an edge. The profile must lie entirely on one side of the chosen axis;
one that straddles it fails recompute with "Revolve axis intersects the
sketch" rather than building a self-intersecting solid.

`mirror new --plane xy|xz|yz` and `pattern linear|polar` copy features instead
of drawing them twice by hand. Both default to the body's tip — the feature
your last verb just left behind — so one fin sketch plus `mirror new --plane
xz` is the whole symmetric pair; name `--feature <name>...` to copy something
older. `pattern linear --direction x|y|z --count N --spacing <MM|PARAM>`
repeats along a line, `--spacing` between each copy and the next, not the
total span; `--reversed` walks the copies the other way along that axis
instead of the direction's positive side — LinearPattern drives a total
length internally, so a negative `--spacing` degenerates rather than turning
the run around, and `--reversed` is the only flag that actually flips it.
`pattern polar --axis x|y|z --count N --angle <DEG|PARAM>` repeats
around an axis, `--angle` the total sweep across every copy, same convention
as `revolve --angle`. `--plane`/`--direction`/`--axis` name a body origin
plane or axis, never a sketch's own — those are body-global, not sketch-local
like `revolve`'s. `--body` and `--document` refuse to guess the same as
everywhere else; so does the direction — there is no default plane or axis to
fall back on. The plane or axis you mirror or pattern across is one of the
body's origin planes, not the sketch's own plane: a fin sketch offset off the
body with `sketch new --offset-z <n>` (the plane's normal, not `--offset-x`/
`--offset-y`, whichever global axis that maps to) sits clear of `xz`, so
mirroring across it produces two fins rather than one folded onto itself.

`fillet new --radius <MM|PARAM> [selection...]` rounds edges and
`chamfer new --size <MM|PARAM> [--angle <DEG|PARAM>] [selection...]` bevels
them, on the body's tip by default — name `--feature <name>` to dress an
earlier feature instead, the same idea as `mirror`/`pattern`'s own
`--feature`, except a dressup's `Base` is a single link so naming more than
one is refused. FreeCAD's own `Body::insertObject` splices the new fillet or
chamfer in right after the named feature and reroutes whatever came next to
build on it instead, so a later feature's own contribution survives — round
the base of a part without losing the boss padded on top of it later. A
failed non-tip dressup rolls the whole splice back, not only the feature it
added: the successor's rerouted link goes back too, the same as any other
creating verb. Neither takes a raw edge name like `Edge12` —
FreeCAD's own edge numbering is the topological-naming problem, it shifts
under any upstream change, so it can never appear in a command a human types.
Edges are selected by geometry instead, through predicates that compose by
AND: `--parallel x|y|z` (edges running along that axis), `--near-min x|y|z` /
`--near-max x|y|z` (edges lying on that face of the tip's own bounding box),
`--longer-than <MM>` / `--shorter-than <MM>`. No predicate at all means every
edge — the same all-edges default FreeCAD's own dressup dialog falls back to.
A selection that matches nothing is refused, not a silent no-op, and the
reply carries `edges_matched` and `edges_length` so you can tell six short
edges from one long one without a second `inspect`. `chamfer`'s `--angle` at
its default (0) is an equal-distance chamfer on `--size` alone, matching how a
zero `--taper` already means "no taper" elsewhere; any other angle switches to
FreeCAD's "Distance and Angle" mode. `document inspect --tree` reports each
dressup's own `radius`/`size` (`chamfer` also shows `angle` when it is in
angled mode) and `edges`, the edge count it resolved to. There is no
`--convex`/`--concave` predicate — geometry-only selection covers the
straightforward cases (a part's outer edges, one face's rim, everything
longer than X); convexity needs comparing adjacent-face normals per edge, a
different and heavier piece of OCCT plumbing than the bounding-box and
direction checks the rest of this uses, so it was left out rather than rushed
in.

### Booleans between bodies

`body union|cut|intersect --tool <BODY>... --base <BODY>` folds one or more
tool bodies into a base body's own PartDesign chain — `--base` defaults only
when exactly one other body exists to be it, and naming the same body as both
is refused. A tool body is not deleted, only reparented: `PartDesign::Boolean`
consumes it into the base's chain, so the tool body's own name and feature
history stay addressable, and `document inspect --tree` keeps listing it —
marked `consumed_by` the boolean feature — even though it no longer stands on
its own. `inspect`'s top-level `solids` count is what actually drops: two live
bodies (or an already-consumed one left dangling) are `ambiguous-shape` to
`preview export`. A body a boolean has already consumed carries no shape of
its own, so it is excluded from every other verb's `--body` guess too — union
two of three bodies together and the one still standing is unambiguous, no
`--body` needed.

`cut` and `intersect` share the same shape but not the same failure: a
disjoint `intersect` is refused as `empty-result` rather than silently
producing a solid with no volume, and the document is left exactly as it was
— nothing is consumed on a refusal, the same rollback discipline as any other
creating verb. `document inspect --tree`'s boolean entry reports `operation`,
`base` (the base body's own feature the chain now runs from, not the body's
name) and `tool` (the consumed bodies' names).

A `union` whose tool only touches the base at a single point or an exact
tangency is a real FreeCAD hazard: the recompute succeeds and the volume is
right, but the result is two solids masquerading as one, and every later
operation on that body returns `Null shape`. `union` checks for this after
recompute and refuses as `degenerate-contact` rather than hand back a body
that is already broken; `cut`/`intersect`/plain `pad`/`pocket`/`revolve`/
`loft` stacking do not carry this check, since a body's own chain can
legitimately leave a gap (see `feature remove`, above) and a gap is not what
this catches.

Documents are not workbench records: nothing here touches the ledger, and a
saved `.FCStd` is only tracked if the human puts it in the repository.

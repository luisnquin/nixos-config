# The proof that the slice works against real FreeCAD: it drives `ee` against a
# server it never starts by hand, then reopens the saved file in a second
# server process and checks the geometry, dimensions and constraints came back.
{
  runCommand,
  jq,
  python3,
  unzip,
  ee-workbench,
}:
runCommand "ee-freecad-slice-test" {
  nativeBuildInputs = [ee-workbench ee-workbench.cad jq python3 unzip];
} ''
  set -euo pipefail

  export HOME=$TMPDIR/home
  export XDG_RUNTIME_DIR=$TMPDIR/run
  export XDG_CACHE_HOME=$TMPDIR/cache
  mkdir -p "$HOME" "$XDG_RUNTIME_DIR"

  socket() {
    export EE_WORKBENCH_CAD_SOCKET=$XDG_RUNTIME_DIR/$1.sock
  }

  # Reads the mesh back the way a slicer would, so a wrong winding or a stale
  # export cannot pass as a fresh one.
  cat >"$TMPDIR/stl.py" <<'EOF'
  import struct, sys

  data = open(sys.argv[1], "rb").read()
  count = struct.unpack("<I", data[80:84])[0]
  assert len(data) == 84 + count * 50, "truncated stl"

  points = []
  for index in range(count):
      at = 84 + index * 50
      points += struct.unpack("<12f", data[at : at + 48])[3:]

  axes = [points[start::3] for start in range(3)]
  print(count, " ".join(f"{max(axis) - min(axis):g}" for axis in axes))
  EOF

  cat >"$TMPDIR/png.py" <<'EOF'
  import struct, sys

  data = open(sys.argv[1], "rb").read()
  assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a png"
  assert data[12:16] == b"IHDR", "no header chunk"
  assert data[-8:-4] == b"IEND", "no end chunk"
  print(*struct.unpack(">II", data[16:24]))
  EOF

  document=$TMPDIR/plate.FCStd

  socket build

  # Nothing has started a server: the first real verb has to.
  ee mechanical status --json | jq -e '.running | not' >/dev/null

  ee mechanical document new --name Plate --json | jq -e '.document == "Plate"' >/dev/null
  ee mechanical status --json | jq -e '
    .running and (.session.freecad.version | test("^[0-9]")) and .session.idle.timeout > 0
  ' >/dev/null

  ee mechanical body new --json | jq -e '.body == "Body"' >/dev/null
  ee mechanical sketch new --plane xy --json | jq -e '
    .plane == "XY_Plane"
    and .basis.normal == {x: 0, y: 0, z: 1}
    and .basis.origin == {x: 0, y: 0, z: 0}
  ' >/dev/null

  ee mechanical sketch rectangle --width 40 --height 25 --json >"$TMPDIR/rectangle.json"
  jq -e '
    (.geometry | length) == 4
    and (.constraints | length) == 12
    and .dof == 0
    and .fully_constrained
    and (.redundant | not)
    and (.centered | not)
    and ([.geometry[].length] | sort) == [25, 25, 40, 40]
  ' "$TMPDIR/rectangle.json" >/dev/null

  ee mechanical pad new --length 6 --json >"$TMPDIR/pad.json"
  jq -e '
    .pad == "Pad"
    and .solid
    and .length.value == 6
    and .bounds == {x: 40, y: 25, z: 6}
    and .shape.volume == 6000
    and .shape.min == {x: 0, y: 0, z: 0}
    and (.recompute.failed | not)
  ' "$TMPDIR/pad.json" >/dev/null

  ee mechanical preview export --json >"$TMPDIR/preview.json"
  jq -e '.object == "Body" and .triangles == 12 and .follow' "$TMPDIR/preview.json" >/dev/null

  stl=$(jq -r '.path' "$TMPDIR/preview.json")
  test "$(python3 "$TMPDIR/stl.py" "$stl")" = "12 40 25 6"

  # The export follows the model: no second preview command is issued here.
  ee mechanical pad length 11 --json | jq -e '.previous == 6 and .bounds.z == 11' >/dev/null
  test "$(python3 "$TMPDIR/stl.py" "$stl")" = "12 40 25 11"

  ee mechanical document recompute --json | jq -e '.failed | not' >/dev/null
  ee mechanical status --json | jq -e '.session.unsaved == ["Plate"] and .session.idle.blocked' >/dev/null

  # Unsaved work outranks a stop: only --force may throw it away.
  if ee mechanical session stop --json >"$TMPDIR/refused.json" 2>"$TMPDIR/refused.err"; then
    echo "stopping with unsaved changes should have been refused" >&2
    exit 1
  fi
  grep -q "unsaved" "$TMPDIR/refused.err"

  ee mechanical document save --path "$document" --json | jq -e --arg f "$document" '.path == $f' >/dev/null
  ee mechanical status --json | jq -e '
    (.session.unsaved | length) == 0 and (.session.idle.blocked | not)
  ' >/dev/null
  ee mechanical session stop --json | jq -e '.stopped' >/dev/null

  test -f "$document"
  unzip -l "$document" | grep -q Document.xml

  # A second process proves the geometry lives in the file, not in the session.
  socket reopen

  # Reopening renames: FreeCAD takes the internal name from the file, not from
  # whatever the document was called when it was created.
  ee mechanical document open --path "$document" --json | jq -e '.document == "plate"' >/dev/null
  ee mechanical document inspect --json >"$TMPDIR/inspect.json"
  ee mechanical session stop --force --json >/dev/null

  jq -e '
    ([.objects[].type] | index("PartDesign::Body")) != null
    and ([.objects[].type] | index("PartDesign::Pad")) != null
    and ([.objects[].type] | index("App::Plane")) == null
    and ([.objects[].type] | index("App::Origin")) == null
    and (.solids | length) == 1
    and .bbox.size == {x: 40, y: 25, z: 11}
  ' "$TMPDIR/inspect.json" >/dev/null

  jq -e '
    [.objects[] | select(.sketch)] as $sketches
    | ($sketches | length) == 1
    | . and ($sketches[0].sketch as $s
        | ($s.geometry | length) == 4
        and ([$s.geometry[].type] | unique) == ["Part::GeomLineSegment"]
        and ([$s.geometry[].length] | sort) == [25, 25, 40, 40]
        and ($s.constraints | length) == 12
        and ([$s.constraints[] | select(.name == "width") | .value]) == [40]
        and ([$s.constraints[] | select(.name == "height") | .value]) == [25]
        and ([$s.constraints[] | select(.name == "x") | .value]) == [0]
        and ([$s.constraints[] | select(.type == "Coincident")] | length) == 4
        and ([$s.constraints[] | select(.type == "Horizontal")] | length) == 2
        and ([$s.constraints[] | select(.type == "Vertical")] | length) == 2
        and $s.dof == 0
        and $s.fully_constrained)
  ' "$TMPDIR/inspect.json" >/dev/null

  # Placement, parameters and the closed vocabulary, in one part: a centred bar
  # and a hole cut from a sketch that sits on the face above it, with the hole
  # following the bar's width through an expression.
  socket placed

  ee mechanical document new --name Cross --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xz --name Side --offset-z 10 --json >"$TMPDIR/side.json"
  jq -e '
    .basis.normal == {x: 0, y: -1, z: 0}
    and .basis.origin == {x: 0, y: -10, z: 0}
  ' "$TMPDIR/side.json" >/dev/null

  ee mechanical sketch new --plane xy --name Plan --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 20 --centered --json >"$TMPDIR/centered.json"
  jq -e '
    .sketch == "Plan"
    and .centered
    and (.geometry | length) == 5
    and ([.geometry[] | select(.construction)] | length) == 1
    and (.constraints | length) == 13
    and ([.constraints[] | select(.type == "Symmetric")] | length) == 1
    and .dof == 0
  ' "$TMPDIR/centered.json" >/dev/null

  ee mechanical pad new --length 6 --json | jq -e '
    .shape.min == {x: -20, y: -10, z: 0} and .shape.centre_of_mass == {x: 0, y: 0, z: 3}
  ' >/dev/null

  # Every dimension is named whether or not anyone asked, so nothing drives
  # them yet and all of them can be adopted. That is the whole of the migration
  # story: a parameter, then a rebind.
  ee mechanical param list --json | jq -e '
    (.parameters | length) == 0
    and ([.orphans[] | select(.object == "Plan") | .slot] | sort)
        == ["height", "width", "x", "y"]
  ' >/dev/null

  ee mechanical param new bar_x 40 --json | jq -e '
    .created and .value == 40 and (.drives | length) == 0
  ' >/dev/null
  ee mechanical sketch set width bar_x --sketch Plan --json | jq -e '
    .value == {value: 40, parameter: "bar_x"} and .previous == 40 and .dof == 0
  ' >/dev/null

  # A slot that took a literal at creation now follows a parameter, and the
  # readback says which one: binding is an edit, not a property of how the
  # dimension was first drawn.
  ee mechanical param list --json | jq -e '
    ([.parameters[].name]) == ["bar_x"]
    and ([.parameters[] | select(.name == "bar_x") | .drives[] | .object + "." + .slot])
        == ["Plan.Constraints.width"]
    and ([.orphans[] | select(.slot == "width")] | length) == 0
  ' >/dev/null

  # A literal over a driven slot is a real intention and a common accident, and
  # they are the same command; only the accident is silent.
  if ee mechanical sketch set width 33 --sketch Plan --json >/dev/null 2>"$TMPDIR/driven.err"; then
    echo "a literal silently replaced a parameter" >&2
    exit 1
  fi
  grep -q "slot-is-driven" "$TMPDIR/driven.err"
  grep -q -- "--unbind" "$TMPDIR/driven.err"

  ee mechanical sketch set width 33 --sketch Plan --unbind --json | jq -e '
    .value == {value: 33, parameter: null} and .previous == 40
  ' >/dev/null
  ee mechanical sketch set width bar_x --sketch Plan --json | jq -e '.value.value == 40' >/dev/null

  # A name that only held until someone drove it would be worse than none: the
  # bar has to stay centred after the width changes.
  ee mechanical param set bar_x 50 --json | jq -e '.value == 50 and .previous == 40' >/dev/null
  ee mechanical document recompute --json | jq -e '.failed | not' >/dev/null
  ee mechanical document inspect --json | jq -e '
    .bbox.min == {x: -25, y: -10, z: 0} and .bbox.max == {x: 25, y: 10, z: 6}
  ' >/dev/null

  # An unnamed sketch means the newest one, so this circle lands on the face.
  # Its position is an expression over the bar's width rather than a number, so
  # the hole is placed relative to an edge that has not stopped moving.
  ee mechanical param new hole_x "=bar_x / 2 - 8" --json | jq -e '
    .value == 17 and .expression == "bar_x / 2 - 8"
  ' >/dev/null
  ee mechanical sketch new --plane xy --name Hole --offset-z 6 --json >/dev/null
  ee mechanical sketch circle --radius 4 --x hole_x --json | jq -e '
    .sketch == "Hole"
    and .dof == 0
    and (.constraints | length) == 3
    and .centre == {x: 17, y: 0}
  ' >/dev/null

  ee mechanical pocket new --length 3 --json >"$TMPDIR/pocket.json"
  jq -e '
    .pocket == "Pocket"
    and .sketch == "Hole"
    and .solid
    and .shape.max == {x: 25, y: 10, z: 6}
    and ((.shape.volume - (6000 - 150.796447)) | fabs) < 0.01
  ' "$TMPDIR/pocket.json" >/dev/null

  # WALL 4 at one remove: driving the head width has to move the hole with it,
  # or the model is a picture of a hammer rather than a hammer.
  ee mechanical param set bar_x 70 --json >"$TMPDIR/driven.json"
  jq -e '
    .value == 70
    and (.recompute.failed | not)
    and ([.drives[] | .object + "." + .slot] | sort)
        == ["Parameters.hole_x", "Plan.Constraints.width"]
  ' "$TMPDIR/driven.json" >/dev/null

  ee mechanical document inspect --features --json >"$TMPDIR/features.json"
  jq -e '
    .bbox.size == {x: 70, y: 20, z: 6}
    and ([.objects[] | select(.error)] | length) == 0
    and (.bodies | length) == 1
    and ([.bodies[0].features[].name]) == ["Pad", "Pocket"]
    and (.bodies[0].features[0] | .kind == "pad" and .sketch.name == "Plan"
         and .sketch.dof == 0 and .error == null
         and (.sketch.dimensions[] | select(.slot == "width") | .parameter) == "bar_x")
    and (.bodies[0].features[1] | .kind == "pocket" and .sketch.name == "Hole"
         and .sketch.dof == 0 and .error == null
         and .sketch.offset.z.value == 6
         and (.sketch.dimensions[] | select(.slot == "x")
              | .value == 27 and .parameter == "hole_x"))
  ' "$TMPDIR/features.json" >/dev/null

  ee mechanical param list --json | jq -e '
    ([.parameters[] | select(.name == "hole_x")
      | .value == 27 and .expression == "bar_x / 2 - 8"]) == [true]
  ' >/dev/null

  # A parameter other slots still use cannot be removed by accident, and
  # --force says which relationships it turned back into numbers.
  if ee mechanical param remove bar_x --json >/dev/null 2>"$TMPDIR/inuse.err"; then
    echo "removing a parameter in use should have been refused" >&2
    exit 1
  fi
  grep -q "parameter-in-use" "$TMPDIR/inuse.err"

  # Change 2: driving a parameter can break features the caller cannot see, so
  # the failure is named and the exit status carries it.
  ee mechanical param new thick 6 --json >/dev/null
  ee mechanical pad length thick --json | jq -e '.value.parameter == "thick"' >/dev/null

  if ee mechanical param set thick 0 --json >"$TMPDIR/broke.json" 2>&1; then
    echo "driving a pad to zero length should have exited nonzero" >&2
    exit 1
  fi
  jq -e '
    .recompute.failed
    and ([.recompute.errors[] | .object]) == ["Pad"]
    and (.recompute.errors[0].status | length) > 0
  ' "$TMPDIR/broke.json" >/dev/null

  # The readback says so too, rather than leaving it to be inferred from a
  # bounding box that looks wrong.
  ee mechanical document inspect --features --json | jq -e '
    [.bodies[0].features[] | select(.error)] | length == 1
  ' >/dev/null

  ee mechanical param set thick 6 --json | jq -e '.recompute.failed | not' >/dev/null
  ee mechanical document inspect --features --json | jq -e '
    [.bodies[0].features[] | select(.error)] | length == 0
  ' >/dev/null

  ee mechanical preview render --path "$TMPDIR/iso.png" --view iso --width 320 --height 240 \
    --json | jq -e '.width == 320 and .height == 240 and .view == "iso" and .triangles > 12' >/dev/null
  test "$(python3 "$TMPDIR/png.py" "$TMPDIR/iso.png")" = "320 240"

  # A relative path belongs to whoever typed it. The server's working directory
  # is this one, so a path resolved server-side would land the file here.
  mkdir -p "$TMPDIR/elsewhere"
  elsewhere=$(cd "$TMPDIR/elsewhere" && pwd -P)
  (cd "$elsewhere" && ee mechanical preview render --path top.png --view top --json) \
    | jq -e --arg d "$elsewhere" '.path == ($d + "/top.png")' >/dev/null
  test -f "$elsewhere/top.png"
  test ! -e top.png

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # Removal: the only verb that makes the model smaller, and so the only one
  # that has to repair links FreeCAD would rather clear. A three-pad stack is
  # the repro - taking the middle one out without relinking rebuilds to the
  # material below the hole and reports up-to-date.
  socket removal

  ee mechanical document new --name Stack --json >/dev/null
  ee mechanical body new --name Stem --json >/dev/null
  ee mechanical param new step 10 --json >/dev/null

  for pair in 1:40 2:30 3:20; do
    index=$(echo "$pair" | cut -d: -f1)
    width=$(echo "$pair" | cut -d: -f2)
    ee mechanical sketch new --plane xy --name "S$index" \
      --offset-z $(( (index - 1) * 10 )) --json >/dev/null
    ee mechanical sketch rectangle --width "$width" --height "$width" --centered --json >/dev/null
    ee mechanical pad new --length step --name "Pad$index" --sketch "S$index" --json >/dev/null
  done

  ee mechanical document inspect --json | jq -e '.solids[0].shape.volume == 29000' >/dev/null

  ee mechanical feature remove Pad2 --dry-run --json >"$TMPDIR/plan.json"
  jq -e '
    .dry_run
    and .removed == "Pad2"
    and ([.relinked[] | .object + "." + .slot + " -> " + .to]) == ["Pad3.BaseFeature -> Pad1"]
    and (.tip_moves | not)
    and ([.left_behind[].object]) == ["S2"]
    and .orphaned == []
    and (.recompute | not)
  ' "$TMPDIR/plan.json" >/dev/null
  ee mechanical document inspect --json | jq -e '.solids[0].shape.volume == 29000' >/dev/null

  # The preview cannot drift from the edit, because it is the same value: the
  # server computes one plan and --dry-run decides only whether to apply it.
  ee mechanical feature remove Pad2 --json >"$TMPDIR/removed.json"
  jq -e --slurpfile plan "$TMPDIR/plan.json" '
    (.dry_run | not)
    and (.recompute.failed | not)
    and (del(.dry_run, .recompute)) == ($plan[0] | del(.dry_run))
  ' "$TMPDIR/removed.json" >/dev/null

  # 20000 and not 4000. Everything else here is bookkeeping; this is the verb.
  ee mechanical document inspect --json | jq -e '.solids[0].shape.volume == 20000' >/dev/null
  ee mechanical document inspect --features --json | jq -e '
    [.bodies[0].features[].name] == ["Pad1", "Pad3"]
  ' >/dev/null

  # The tip follows the feature under it, and the slot that went away leaves
  # the drives index with it rather than naming a dead object.
  ee mechanical param list --json >"$TMPDIR/before.json"
  jq -e '
    ([.parameters[] | select(.name == "step") | .drives[] | .object + "." + .slot] | sort)
      == ["Pad1.Length", "Pad3.Length"]
  ' "$TMPDIR/before.json" >/dev/null

  ee mechanical feature remove Pad3 --json >"$TMPDIR/tip.json"
  jq -e '.tip_moves and .tip == "Pad1" and ([.left_behind[].object]) == ["S3"]' \
    "$TMPDIR/tip.json" >/dev/null
  ee mechanical document inspect --json | jq -e '.solids[0].shape.volume == 16000' >/dev/null
  ee mechanical param list --json | jq -e '
    ([.parameters[] | select(.name == "step") | .drives[] | .object + "." + .slot])
      == ["Pad1.Length"]
  ' >/dev/null

  # Build, remove, rebuild. Geometry and the whole parameter list come back
  # identical, which is the only evidence that nothing was left dangling.
  ee mechanical pad new --length step --name Pad3 --sketch S3 --json >/dev/null
  ee mechanical document inspect --json | jq -e '.solids[0].shape.volume == 20000' >/dev/null
  ee mechanical param list --json >"$TMPDIR/after.json"
  diff <(jq -S . "$TMPDIR/before.json") <(jq -S . "$TMPDIR/after.json")

  # A sketch a live feature still draws from refuses. Removing it anyway leaves
  # the holder with a null profile, an Invalid status and the shape it last
  # built, which looks right and cannot be rebuilt.
  if ee mechanical feature remove S1 --json >/dev/null 2>"$TMPDIR/inuse.err"; then
    echo "removing a profile in use should have been refused" >&2
    exit 1
  fi
  grep -q "sketch-in-use" "$TMPDIR/inuse.err"
  grep -q "Pad1" "$TMPDIR/inuse.err"

  # The widowed one goes, which is why removal leaves profiles behind instead
  # of taking them: cleaning up is a second command, not a policy.
  ee mechanical feature remove S2 --json | jq -e '.removed == "S2"' >/dev/null

  # A body is not a feature. What removing one means depends on whether another
  # is built from it, so it waits for booleans rather than guessing now.
  if ee mechanical feature remove Stem --json >/dev/null 2>"$TMPDIR/body.err"; then
    echo "removing a body should have been refused" >&2
    exit 1
  fi
  grep -q "unremovable" "$TMPDIR/body.err"

  # Arithmetic written over a feature is the one reference removal cannot
  # repair, and rewriting somebody's expression is not this verb's business.
  # The pad has to hold a literal first, or the parameter would be a cycle.
  ee mechanical pad length 10 --pad Pad1 --unbind --json >/dev/null
  ee mechanical param new echoed "=Pad1.Length * 2" --json | jq -e '.value == 20' >/dev/null
  ee mechanical param new tripled "=Pad1.Length * 3" --json | jq -e '.value == 30' >/dev/null
  if ee mechanical feature remove Pad1 --dry-run --json >/dev/null 2>"$TMPDIR/follows.err"; then
    echo "removing a feature a parameter reads should have been refused" >&2
    exit 1
  fi
  # Both of them, in one refusal. A refusal that names a sample costs one round
  # trip per follower to discover a set the server already had in hand.
  grep -q "parameter-follows" "$TMPDIR/follows.err"

  # Membership first, per name and order-independently, because that is the
  # promise the message actually makes.
  grep -q "echoed" "$TMPDIR/follows.err"
  grep -q "tripled" "$TMPDIR/follows.err"

  # Then the order, as its own promise with its own reason: getExpressions() is
  # keyed by ObjectIdentifier so the set arrives sorted, and pinning it keeps
  # the refusal diffable across runs rather than incidentally stable.
  grep -q "echoed, tripled" "$TMPDIR/follows.err"
  ee mechanical param set echoed 20 --json >/dev/null
  ee mechanical param set tripled 30 --json >/dev/null

  # Emptying a body is legal, and the parameter that drove its last slot
  # survives driving nothing rather than being deleted on the model's behalf.
  ee mechanical feature remove Pad3 --json | jq -e '.orphaned == ["step"]' >/dev/null
  ee mechanical feature remove Pad1 --json | jq -e '.tip_moves and .tip == null' >/dev/null
  ee mechanical param list --json | jq -e '
    ([.parameters[].name] | sort) == ["echoed", "step", "tripled"]
    and ([.parameters[] | select(.drives | length > 0)] | length) == 0
  ' >/dev/null

  # No tip is no shape, not a small one: the body stops being a solid at all.
  ee mechanical document inspect --json | jq -e '.solids == [] and (.bbox | not)' >/dev/null
  if ee mechanical preview export --path "$TMPDIR/empty.stl" --json >/dev/null 2>&1; then
    echo "exporting an emptied body should have been refused" >&2
    exit 1
  fi

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # Registry integrity. A parameter's number and its own expression have to
  # agree, or the listing has to say which of the two is not to be trusted -
  # one failing expression aborts the whole VarSet, so a stale row is the
  # normal case rather than a corner.
  socket registry

  ee mechanical document new --name Poison --json >/dev/null
  ee mechanical body new --name B --json >/dev/null
  ee mechanical sketch new --plane xy --name S --json >/dev/null
  ee mechanical sketch rectangle --width 10 --height 10 --centered --json >/dev/null
  ee mechanical pad new --length 10 --name Pad1 --sketch S --json >/dev/null

  # A bind that does not evaluate leaves nothing behind, the registry itself
  # included. Every other refusal in this tool leaves the document untouched,
  # and a caller reading a nonzero exit as "nothing happened" has to be right.
  ee mechanical document inspect --features --json >"$TMPDIR/pristine.json"
  if ee mechanical param new broken "=Pad1.Length + 1" --json >/dev/null 2>"$TMPDIR/bind.err"; then
    echo "a unit mismatch should have been refused" >&2
    exit 1
  fi
  grep -q "invalid-expression" "$TMPDIR/bind.err"
  ee mechanical document inspect --features --json >"$TMPDIR/rolled.json"
  diff <(jq -S . "$TMPDIR/pristine.json") <(jq -S . "$TMPDIR/rolled.json")
  ee mechanical param list --json | jq -e '.parameters == []' >/dev/null

  # And a refused `param set` puts back the expression it replaced, not merely
  # the number: restoring the value alone would leave the row computing again
  # from arithmetic nobody chose.
  ee mechanical param new z 5 --json >/dev/null
  ee mechanical param new keep "=z + 7" --json | jq -e '.value == 12' >/dev/null
  if ee mechanical param set keep "=z + nosuch" --json >/dev/null 2>&1; then
    echo "an expression naming nothing should have been refused" >&2
    exit 1
  fi
  ee mechanical param list --json | jq -e '
    [.parameters[] | select(.name == "keep")][0]
    | .value == 12 and .expression == "z + 7" and .state == "ok"
  ' >/dev/null

  # All three states at once. abad sorts before zgood, and that is exactly what
  # decides whether the aborted recompute ever reached zgood - so the same
  # broken registry labels a row differently depending on its name, which is
  # the reason the flag cannot be left to the reader to infer.
  ee mechanical param new abad "=10 / z" --json | jq -e '.value == 2' >/dev/null
  ee mechanical param new zgood "=z + 100" --json | jq -e '.value == 105' >/dev/null
  if ee mechanical param set z 0 --json >/dev/null 2>&1; then
    echo "a set that stops another expression should not report success" >&2
    exit 1
  fi

  ee mechanical param list --json >"$TMPDIR/states.json" 2>/dev/null || true
  jq -e '
    ([.parameters[] | select(.name == "z")][0].state == "ok")
    and ([.parameters[] | select(.name == "abad")][0].state == "invalid")
    and ([.parameters[] | select(.name == "zgood")][0].state == "not-evaluated")
  ' "$TMPDIR/states.json" >/dev/null

  # The culprit carries FreeCAD's own diagnostic through to the machine
  # surface, whole and unparsed.
  jq -e '
    [.parameters[] | select(.name == "abad")][0].error | test("division by zero")
  ' "$TMPDIR/states.json" >/dev/null

  # The stale row keeps its last good number rather than zeroing, which is why
  # nothing but the flag tells it apart from a live one: 105 is what z + 100
  # produced when z was 5, and z is 0.
  jq -e '[.parameters[] | select(.name == "zgood")][0].value == 105' "$TMPDIR/states.json" \
    >/dev/null

  # The human surface says both things too, and the listing is the one surface
  # a caller reaches for precisely when something already looks wrong.
  ee mechanical param list >"$TMPDIR/table.txt" 2>&1 || true
  grep -q "not-evaluated" "$TMPDIR/table.txt"
  grep -q "stopped the registry" "$TMPDIR/table.txt"
  grep -q "division by zero" "$TMPDIR/table.txt"

  if ee mechanical param list --json >/dev/null 2>&1; then
    echo "param list should exit nonzero while an expression fails" >&2
    exit 1
  fi

  # Repaired, everything clears, and the column disappears with it: a state
  # reading ok on every row of a healthy registry is noise.
  ee mechanical param set z 5 --json >/dev/null
  ee mechanical param list --json | jq -e '
    ([.parameters[] | select(.state != "ok")] | length) == 0
    and ([.parameters[] | select(.name == "zgood")][0].value == 105)
  ' >/dev/null
  ee mechanical param list >"$TMPDIR/healthy.txt"
  if head -1 "$TMPDIR/healthy.txt" | grep -q "state"; then
    echo "a healthy registry should not carry a state column" >&2
    exit 1
  fi

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # The profile vocabulary: several primitives in one sketch, lines and arcs
  # closing a wire, a polyline, and an outer loop padded with its inner loops
  # in one operation. Volumes are checked against closed-form arithmetic - a
  # wrong model is usually wrong in the bounding box first, and after that in
  # the volume.
  socket profiles

  ee mechanical document new --name Profile --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null

  # Three lines and an arc close one wire. Every primitive lands fully
  # constrained on its own, and the second one's dimensions arrive suffixed:
  # "x1" is taken, so the arc's endpoints answer to x1_2 and friends.
  ee mechanical sketch line --x1 0 --y1 0 --x2 30 --y2 0 --json | jq -e '
    .dof == 0 and .slots == {x1: "x1", y1: "y1", x2: "x2", y2: "y2"}
  ' >/dev/null
  ee mechanical sketch arc --x1 30 --y1 0 --x2 30 --y2 20 --radius 12 --json >"$TMPDIR/arc.json"
  jq -e '
    .dof == 0
    and .slots.radius == "radius" and .slots.x1 == "x1_2"
    and ((.centre.x - 23.36675) | fabs) < 0.001 and .centre.y == 10
  ' "$TMPDIR/arc.json" >/dev/null
  ee mechanical sketch line --x1 30 --y1 20 --x2 0 --y2 20 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical sketch line --x1 0 --y1 20 --x2 0 --y2 0 --json | jq -e '
    .dof == 0 and .slots.x1 == "x1_4"
  ' >/dev/null

  # A radius too small to span its chord is a refusal, not a NaN centre.
  if ee mechanical sketch arc --x1 0 --y1 0 --x2 30 --y2 0 --radius 5 --json \
      >/dev/null 2>"$TMPDIR/short.err"; then
    echo "an arc radius shorter than half the chord should have been refused" >&2
    exit 1
  fi
  grep -q "invalid-dimension" "$TMPDIR/short.err"

  # 30x20 rectangle plus the circular segment the arc bulges out:
  # r=12, chord=20 -> theta = 2*asin(10/12), segment = r^2/2*(theta - sin theta)
  # volume = (600 + 75.520744) * 5, max x = 30 - sqrt(44) + 12.
  ee mechanical pad new --length 5 --json >"$TMPDIR/profile-pad.json"
  jq -e '
    .solid
    and ((.shape.volume - 3377.617285) | fabs) < 0.001
    and ((.shape.max.x - 35.36675) | fabs) < 0.001
    and .shape.max.y == 20 and .shape.max.z == 5
  ' "$TMPDIR/profile-pad.json" >/dev/null

  # The suffixed dimension is a real slot: bind the arc radius, drive it, and
  # the solid follows. r=14 -> volume (600 + 57.958692) * 5.
  ee mechanical param new bulge 12 --json >/dev/null
  ee mechanical sketch set radius bulge --json | jq -e '.value.parameter == "bulge"' >/dev/null
  ee mechanical param set bulge 14 --json | jq -e '.recompute.failed | not' >/dev/null
  ee mechanical document inspect --json | jq -e '
    ((.solids[0].shape.volume - 3289.792946) | fabs) < 0.001
    and ((.bbox.max.x - 34.202041) | fabs) < 0.001
  ' >/dev/null

  # A plate with four holes out of ONE sketch and ONE pocket, where it used to
  # cost four sketches and four pockets. Each circle names its own dimensions.
  ee mechanical document new --name Plate --json >/dev/null
  ee mechanical body new --document Plate --json >/dev/null
  ee mechanical sketch new --plane xy --name Base --document Plate --json >/dev/null
  ee mechanical sketch rectangle --width 60 --height 40 --document Plate --json >/dev/null
  ee mechanical pad new --length 5 --document Plate --json >/dev/null
  ee mechanical sketch new --plane xy --offset-z 5 --name Holes --document Plate --json >/dev/null
  ee mechanical sketch circle --radius 3 --x 10 --y 10 --sketch Holes --document Plate --json >/dev/null
  ee mechanical sketch circle --radius 3 --x 50 --y 10 --sketch Holes --document Plate --json >/dev/null
  ee mechanical sketch circle --radius 3 --x 50 --y 30 --sketch Holes --document Plate --json >/dev/null
  ee mechanical sketch circle --radius 3 --x 10 --y 30 --sketch Holes --document Plate --json \
    | jq -e '.dof == 0 and .slots == {radius: "radius_4", x: "x_4", y: "y_4"}' >/dev/null
  ee mechanical pocket new --through-all --sketch Holes --document Plate --json >/dev/null

  # 60*40*5 - 4*pi*9*5, and every suffixed dimension shows up as an orphan the
  # registry can adopt.
  ee mechanical document inspect --document Plate --json | jq -e '
    ((.solids[0].shape.volume - 11434.513322) | fabs) < 0.001
  ' >/dev/null
  ee mechanical param list --document Plate --json | jq -e '
    ([.orphans[] | select(.object == "Holes") | .slot] | sort)
      == ["radius", "radius_2", "radius_3", "radius_4",
          "x", "x_2", "x_3", "x_4", "y", "y_2", "y_3", "y_4"]
  ' >/dev/null

  # A closed polyline with a parameter-driven vertex: 40-wide triangle whose
  # apex follows `apex`, so driving it moves the padded solid.
  ee mechanical document new --name Tri --json >/dev/null
  ee mechanical body new --document Tri --json >/dev/null
  ee mechanical sketch new --plane xy --document Tri --json >/dev/null
  ee mechanical param new apex 25 --document Tri --json >/dev/null
  ee mechanical sketch polyline --points "0,0 40,0 20,apex" --close --document Tri --json \
    | jq -e '
      .dof == 0 and .closed and (.points | length) == 3
      and .slots == {x1: "x1", y1: "y1", x2: "x2", y2: "y2", x3: "x3", y3: "y3"}
    ' >/dev/null
  ee mechanical pad new --length 4 --document Tri --json | jq -e '
    .solid and ((.shape.volume - 2000) | fabs) < 0.001
  ' >/dev/null
  ee mechanical param set apex 30 --document Tri --json | jq -e '.recompute.failed | not' >/dev/null
  ee mechanical document inspect --document Tri --json | jq -e '
    ((.solids[0].shape.volume - 2400) | fabs) < 0.001 and .bbox.max.y == 30
  ' >/dev/null

  # An open polyline cannot close a wire and --close needs three points; both
  # are refusals at the sketch, not a broken pad later.
  if ee mechanical sketch polyline --points "0,0 10,0" --close --document Tri --json \
      >/dev/null 2>"$TMPDIR/two.err"; then
    echo "a closed polyline of two points should have been refused" >&2
    exit 1
  fi
  grep -q "invalid-dimension" "$TMPDIR/two.err"

  # An outer loop and an inner loop in the same sketch pad to a solid with a
  # hole in one operation: 30*30*6 - pi*25*6.
  ee mechanical document new --name Ring --json >/dev/null
  ee mechanical body new --document Ring --json >/dev/null
  ee mechanical sketch new --plane xy --document Ring --json >/dev/null
  ee mechanical sketch rectangle --width 30 --height 30 --document Ring --json >/dev/null
  ee mechanical sketch circle --radius 5 --x 15 --y 15 --document Ring --json >/dev/null
  ee mechanical pad new --length 6 --document Ring --json | jq -e '
    .solid and ((.shape.volume - 4928.761102) | fabs) < 0.001
  ' >/dev/null

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # An idle session goes away by itself, but never with work in it.
  socket idle
  export EE_WORKBENCH_CAD_IDLE=2

  ee mechanical document new --name Keep --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 10 --height 10 --json >/dev/null
  sleep 6
  ee mechanical status --json | jq -e '.running and .session.idle.blocked' >/dev/null

  ee mechanical document save --path "$TMPDIR/keep.FCStd" --json >/dev/null
  sleep 6
  ee mechanical status --json | jq -e '.running | not' >/dev/null

  # A cold start again, and the positional spelling of the path this time.
  socket final
  unset EE_WORKBENCH_CAD_IDLE
  ee mechanical document open "$TMPDIR/keep.FCStd" --json | jq -e '.document == "keep"' >/dev/null

  # The whole pairing in one assertion: `ee` is the wrapper, so the server it
  # spawned is the one its own store path names, and the server says so itself.
  ee mechanical status --json \
    | jq -e '.build.stale == false and (.build.running | length > 0)
             and .build.running == .build.expected' >/dev/null

  # What a session left behind by an older generation looks like. The wrapper
  # pins the expectation on purpose, so the drift has to be staged through the
  # unwrapped client, which is the one that still reads its environment.
  export EE_WORKBENCH_CAD_BUILD=/nix/store/an-older-generation
  plain=${ee-workbench.client}/bin/ee

  if refused=$($plain mechanical body new --name Nope --json 2>&1); then
    echo "a session from another build accepted a mutation: $refused" >&2
    exit 1
  fi
  grep -q "older generation" <<<"$refused"

  # Seeing it, saving out of it and stopping it are the way back out.
  $plain mechanical status --json | jq -e '.build.stale' >/dev/null
  $plain mechanical document save --path "$TMPDIR/rescued.FCStd" --json >/dev/null
  test -f "$TMPDIR/rescued.FCStd"
  unset EE_WORKBENCH_CAD_BUILD

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # Geometry flags take a unit-bearing expression through the same grammar a
  # parameter's own expression takes, not just a bare millimetre number.
  socket units

  ee mechanical document new --name Units --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null

  ee mechanical sketch circle --radius "2 m" --x 0 --y 0 --json | jq -e '.radius.value == 2000' >/dev/null
  ee mechanical sketch circle --radius "1 in" --x 10 --y 0 --json | jq -e '.radius.value == 25.4' >/dev/null
  ee mechanical sketch circle --radius "5 cm / 2" --x 20 --y 0 --json | jq -e '.radius.value == 25' >/dev/null
  ee mechanical sketch circle --radius "12.7 mm + 1 in" --x 30 --y 0 --json \
    | jq -e '.radius.value == 38.1' >/dev/null
  ee mechanical sketch circle --radius 5 --x 40 --y 0 --json | jq -e '.radius.value == 5' >/dev/null

  # A positional expression is allowed to start with a minus: clap must not eat
  # it as a flag.
  ee mechanical param new head_len 40 --json >/dev/null
  ee mechanical param new claw_len 12 --json >/dev/null
  ee mechanical param new claw_x "-(head_len/2 - claw_len/2)" --json \
    | jq -e '.value == -14' >/dev/null

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # Revolve and groove: the round twin of pad and pocket. A rectangle touching
  # its own revolve axis sweeps into a solid cylinder, volume pi*r^2*h.
  socket revolve

  ee mechanical document new --name Cylinder --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 8 --height 20 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical revolve new --angle 360 --axis y --json | jq -e '
    .solid and ((.shape.volume - 4021.238597) | fabs) < 0.001
    and .shape.max.x == 8 and .shape.min.x == -8
  ' >/dev/null

  # `--axis x` sweeps about the sketch's other in-plane axis: same volume,
  # the bounding box turns through ninety degrees around it instead.
  ee mechanical document new --name AxisX --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 8 --json >/dev/null
  ee mechanical revolve new --angle 360 --axis x --json | jq -e '
    .solid and ((.shape.volume - 4021.238597) | fabs) < 0.001
    and .shape.max.x == 20 and .shape.min.x == 0
    and .shape.max.y == 8 and .shape.min.y == -8
  ' >/dev/null

  # A circle away from the axis revolves into a torus, volume 2*pi^2*R*r^2 -
  # proof the wave-1 profile vocabulary and revolve compose, not just a
  # rectangle. Retargeting the angle like `pad length` and recomputing halves
  # it, and a named parameter drives the same slot.
  ee mechanical document new --name Torus --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch circle --radius 5 --x 20 --y 0 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical revolve new --angle 360 --axis y --json | jq -e '
    .solid and ((.shape.volume - 9869.604401) | fabs) < 0.001
  ' >/dev/null
  ee mechanical param new sweep 180 --json >/dev/null
  ee mechanical revolve angle sweep --json | jq -e '.value.parameter == "sweep" and .previous == 360' >/dev/null
  ee mechanical document inspect --json | jq -e '
    ((.solids[0].shape.volume - 4934.802201) | fabs) < 0.001
  ' >/dev/null
  ee mechanical param set sweep 360 --json | jq -e '.recompute.failed | not' >/dev/null
  ee mechanical document inspect --json | jq -e '
    ((.solids[0].shape.volume - 9869.604401) | fabs) < 0.001
  ' >/dev/null

  # An angle outside (0, 360] is refused before a feature is built, the same
  # class of error as a bad length.
  if ee mechanical revolve new --angle 400 --axis y --json >/dev/null 2>"$TMPDIR/angle.err"; then
    echo "an angle outside (0, 360] should have been refused" >&2
    exit 1
  fi
  grep -q "invalid-dimension" "$TMPDIR/angle.err"

  # Groove is the subtractive twin: a concentric smaller circle revolved into
  # the same torus removes exactly the smaller torus, 2*pi^2*R*(r1^2 - r2^2).
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch circle --radius 2 --x 20 --y 0 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical groove new --angle 360 --axis y --json | jq -e '
    .solid and ((.shape.volume - 8290.467697) | fabs) < 0.001
  ' >/dev/null

  # Groove refuses to cut a body with nothing in it yet, the same way pocket does.
  ee mechanical document new --name EmptyGroove --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch circle --radius 5 --json >/dev/null
  if ee mechanical groove new --angle 360 --axis y --json >/dev/null 2>"$TMPDIR/nomat.err"; then
    echo "a groove on an empty body should have been refused" >&2
    exit 1
  fi
  grep -q "no-material" "$TMPDIR/nomat.err"

  # Both angle and taper are named parameter slots like every other dimension.
  ee mechanical param list --document Torus --json | jq -e '
    ([.parameters[] | select(.name == "sweep") | .drives[0]])
      == [{object: "Revolution", slot: "Angle"}]
  ' >/dev/null

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # `--taper` on pad and pocket: a native draft angle, no edge selection.
  # Cross-section at height z is a square of side (20 + 2*z*tan(5deg)); its
  # integral 0..30 is the pad's volume.
  socket taper

  ee mechanical document new --name Taper --json >/dev/null
  ee mechanical body new --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --centered --json >/dev/null
  ee mechanical param new draft 5 --json >/dev/null
  ee mechanical pad new --length 30 --taper draft --json | jq -e '
    .solid and .taper.parameter == "draft"
    and ((.shape.volume - 15425.145472) | fabs) < 0.001
    and ((.shape.max.x - 12.62466) | fabs) < 0.001
  ' >/dev/null
  ee mechanical param list --json | jq -e '
    ([.parameters[] | select(.name == "draft") | .drives[0]])
      == [{object: "Pad", slot: "TaperAngle"}]
  ' >/dev/null

  ee mechanical sketch new --plane xy --offset-z 30 --json >/dev/null
  ee mechanical sketch rectangle --width 10 --height 10 --centered --json >/dev/null
  ee mechanical pocket new --length 8 --taper 3 --json | jq -e '
    .solid and .taper.value == 3
    and ((.shape.volume - 14556.188519) | fabs) < 0.001
  ' >/dev/null

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # Mirror and pattern: the vocabulary that keeps a symmetric or repeated part
  # from being drawn by hand, hole by hole or fin by fin, with negated
  # coordinates that stop matching the moment a parameter changes.
  socket transform

  # One fin sketch off the XZ origin plane, mirrored across it. Driving the
  # sketch's own width moves both halves, because there is only one sketch.
  ee mechanical document new --name Wing --json >/dev/null
  ee mechanical body new --name Wing --json >/dev/null
  ee mechanical param new fin_len 20 --json >/dev/null
  ee mechanical sketch new --plane xz --offset-z 4 --json | jq -e '
    .basis.normal == {x: 0, y: -1, z: 0} and .basis.origin == {x: 0, y: -4, z: 0}
  ' >/dev/null
  ee mechanical sketch rectangle --width fin_len --height 6 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical pad new --length 2 --json | jq -e '
    .shape.volume == 240 and .shape.min == {x: 0, y: -6, z: 0} and .shape.max == {x: 20, y: -4, z: 6}
  ' >/dev/null

  ee mechanical mirror new --plane xz --json >"$TMPDIR/mirror.json"
  jq -e '
    .mirror == "Mirrored" and .plane == "XZ_Plane" and .features == ["Pad"]
    and .shape.volume == 480
    and .shape.min == {x: 0, y: -6, z: 0} and .shape.max == {x: 20, y: 6, z: 6}
  ' "$TMPDIR/mirror.json" >/dev/null

  ee mechanical param set fin_len 30 --json | jq -e '.recompute.failed | not' >/dev/null
  ee mechanical document inspect --json | jq -e '
    .solids[0].shape.volume == 720
    and .solids[0].shape.max == {x: 30, y: 6, z: 6}
  ' >/dev/null

  # `--tree` reports what each feature contributed, not the running total: the
  # mirror's own delta is the second fin alone, not both.
  ee mechanical document inspect --tree --json >"$TMPDIR/tree.json"
  jq -e '
    .bodies[0].features[0].name == "Pad" and .bodies[0].features[0].volume_delta == 360
    and .bodies[0].features[0].sketch.basis.origin == {x: 0, y: -4, z: 0}
    and (.bodies[0].features[0].sketch.primitives | length) == 4
    and .bodies[0].features[1].name == "Mirrored" and .bodies[0].features[1].kind == "mirror"
    and .bodies[0].features[1].volume_delta == 360
    and .bodies[0].features[1].bbox_delta == {x: 0, y: 10, z: 0}
  ' "$TMPDIR/tree.json" >/dev/null

  # No default plane or axis to fall back on, the same discipline as `--body`.
  if ee mechanical mirror new --json >/dev/null 2>"$TMPDIR/noplane.err"; then
    echo "mirror without --plane should have been refused" >&2
    exit 1
  fi

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # A bolt circle: one off-centre hole, polar-patterned six times around the
  # body's own Z axis. The removed volume has to be exactly six times the one
  # hole's own delta, not a mesh-derived approximation.
  socket pattern

  ee mechanical document new --name Bolts --json >/dev/null
  ee mechanical body new --name Plate --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 60 --height 60 --centered --json >/dev/null
  ee mechanical pad new --length 5 --json | jq -e '.shape.volume == 18000' >/dev/null

  ee mechanical sketch new --plane xy --offset-z 5 --json >/dev/null
  ee mechanical sketch circle --radius 3 --x 20 --y 0 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical pocket new --through-all --json >"$TMPDIR/hole.json"
  jq -e '((.shape.volume - 17858.628331) | fabs) < 0.001' "$TMPDIR/hole.json" >/dev/null

  ee mechanical pattern polar --axis z --count 6 --json >"$TMPDIR/polar.json"
  jq -e '
    .pattern == "PolarPattern" and .axis == "z" and .count == 6
    and .angle.value == 360 and .features == ["Pocket"]
    and ((.shape.volume - 17151.769984) | fabs) < 0.001
  ' "$TMPDIR/polar.json" >/dev/null

  # 6 * the one hole's own delta == everything the pattern plus its original
  # removed, read straight off the tree's own per-feature deltas rather than
  # recomputed by hand: -141.371669 * 6 == -706.858347 + -141.371669.
  ee mechanical document inspect --tree --json >"$TMPDIR/booltree.json"
  jq -e '
    .bodies[0].features[1].name == "Pocket" and .bodies[0].features[1].volume_delta == -141.371669
    and .bodies[0].features[2].name == "PolarPattern" and .bodies[0].features[2].kind == "pattern_polar"
    and .bodies[0].features[2].volume_delta == -706.858347
    and ((.bodies[0].features[1].volume_delta * 6 - .bodies[0].features[1].volume_delta
          - .bodies[0].features[2].volume_delta) | fabs) < 0.001
  ' "$TMPDIR/booltree.json" >/dev/null

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # A named, parameter-driven spacing: three copies of one hole walking off in
  # X inside a bar with room for all of them, `--spacing` between consecutive
  # copies rather than the total run.
  socket linear

  ee mechanical document new --name Rail --json >/dev/null
  ee mechanical body new --name Rail --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 100 --height 20 --json >/dev/null
  ee mechanical pad new --length 5 --json | jq -e '.shape.volume == 10000' >/dev/null

  ee mechanical sketch new --plane xy --offset-z 5 --json >/dev/null
  ee mechanical sketch circle --radius 2 --x 10 --y 10 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical pocket new --through-all --json >"$TMPDIR/rail-hole.json"
  jq -e '((.shape.volume - (10000 - 62.831853)) | fabs) < 0.001' "$TMPDIR/rail-hole.json" >/dev/null

  ee mechanical param new step 15 --json >/dev/null
  ee mechanical pattern linear --direction x --count 3 --spacing step --json >"$TMPDIR/linear.json"
  jq -e '
    .pattern == "LinearPattern" and .direction == "x" and .count == 3
    and .spacing.value == 15 and .spacing.parameter == "step" and .features == ["Pocket"]
    and ((.shape.volume - (10000 - 3 * 62.831853)) | fabs) < 0.001
  ' "$TMPDIR/linear.json" >/dev/null

  # No default plane or axis to fall back on, the same discipline as `--body`.
  if ee mechanical pattern linear --direction x --count 3 --spacing 5 --json \
      >/dev/null 2>"$TMPDIR/nodirection.err"; then
    echo "pattern linear without a resolved direction should have been refused" >&2
    exit 1
  fi

  # `--reversed` walks the copies the other way: a hole near the rail's far end
  # copies back toward the material it still has, not off the end of it.
  ee mechanical sketch new --plane xy --offset-z 5 --json >/dev/null
  ee mechanical sketch circle --radius 2 --x 95 --y 10 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical pocket new --through-all --json | jq -e '
    ((.shape.volume - (10000 - 4 * 62.831853)) | fabs) < 0.001
  ' >/dev/null

  ee mechanical pattern linear --direction x --count 2 --spacing 8 --reversed --json \
    >"$TMPDIR/reversed.json"
  jq -e '
    (.features | length) == 1
    and ((.shape.volume - (10000 - 5 * 62.831853)) | fabs) < 0.001
  ' "$TMPDIR/reversed.json" >/dev/null

  # A negative spacing is a value clap must accept, not eat as an unknown flag -
  # FreeCAD's own LinearPattern still refuses the degenerate total length it
  # produces, and that refusal has to leave the tree exactly as it found it.
  before=$(ee mechanical document inspect --features --json | jq '[.bodies[0].features[].name] | length')
  if ee mechanical pattern linear --direction x --count 2 --spacing -8 --json \
      >/dev/null 2>"$TMPDIR/negative.err"; then
    echo "a degenerate negative-spacing pattern should have been refused" >&2
    exit 1
  fi
  grep -q "recompute-failed" "$TMPDIR/negative.err"
  after=$(ee mechanical document inspect --features --json | jq '[.bodies[0].features[].name] | length')
  test "$before" = "$after"

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # Fillet and chamfer: edges are matched by geometry, never a raw `EdgeN` name
  # - that naming is FreeCAD's topological naming and shifts under any upstream
  # change, so it must never appear in a request. Predicates compose by AND and
  # default to every edge; a selection that matches nothing is refused, not a
  # silent no-op.
  socket dressup

  ee mechanical document new --name FilletZ --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 30 --json >/dev/null
  ee mechanical pad new --length 10 --json | jq -e '.shape.volume == 12000' >/dev/null

  # The four vertical edges, rounded: each corner loses a quarter-cylinder,
  # r^2 * (4 - pi) * h over all four.
  ee mechanical fillet new --radius 2 --parallel z --json >"$TMPDIR/fillet.json"
  jq -e '
    .fillet == "Fillet" and .base == "Pad" and .edges_matched == 4
    and .edges_length == 40 and .radius.value == 2 and (.recompute.failed | not)
    and ((.shape.volume - 11965.663706) | fabs) < 0.001
  ' "$TMPDIR/fillet.json" >/dev/null

  ee mechanical document inspect --tree --json | jq -e '
    .bodies[0].features[1].kind == "fillet" and .bodies[0].features[1].edges == 4
    and .bodies[0].features[1].radius.value == 2
  ' >/dev/null

  # One isolated top edge - parallel to x, lying on the z-max face and on the
  # y-min side of it - picks exactly one, with no edge left beside it to
  # interact with: an equal-distance chamfer of it is a plain wedge,
  # size^2/2 * length. `--near-min` and `--near-max` are separate flags, so
  # composing two axis predicates at once still means one occurrence each.
  ee mechanical document new --name ChamferEdge --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 30 --json >/dev/null
  ee mechanical pad new --length 10 --json >/dev/null

  ee mechanical chamfer new --size 2 --parallel x --near-max z --near-min y --json \
    >"$TMPDIR/chamfer.json"
  jq -e '
    .chamfer == "Chamfer" and .base == "Pad" and .edges_matched == 1
    and .edges_length == 40 and .size.value == 2 and (has("angle") | not)
    and (.recompute.failed | not) and ((.shape.volume - 11920) | fabs) < 0.001
  ' "$TMPDIR/chamfer.json" >/dev/null

  # An angle switches the dressup into "Distance and Angle" mode, and the tree
  # carries it back out.
  ee mechanical document inspect --tree --json | jq -e '
    .bodies[0].features[1].kind == "chamfer" and .bodies[0].features[1].edges == 1
    and .bodies[0].features[1].size.value == 2
  ' >/dev/null

  # Every other predicate, on the same 40x30x10 box shape: x-edges are 40 long,
  # y-edges 30, z-edges 10, so `--longer-than`/`--shorter-than`/`--near-min`
  # each pick out an exact, countable set.
  ee mechanical document new --name LongEdges --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 30 --json >/dev/null
  ee mechanical pad new --length 10 --json >/dev/null
  ee mechanical fillet new --radius 1 --longer-than 35 --json | jq -e '
    .edges_matched == 4 and .edges_length == 160
  ' >/dev/null

  ee mechanical document new --name BottomEdges --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 30 --json >/dev/null
  ee mechanical pad new --length 10 --json >/dev/null
  ee mechanical fillet new --radius 1 --near-min z --json | jq -e '
    .edges_matched == 4 and .edges_length == 140
  ' >/dev/null

  ee mechanical document new --name NoMatch --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 30 --json >/dev/null
  ee mechanical pad new --length 10 --json >/dev/null
  if ee mechanical fillet new --radius 1 --shorter-than 5 --json \
      >/dev/null 2>"$TMPDIR/nomatch.err"; then
    echo "a selection matching no edge should have been refused" >&2
    exit 1
  fi
  grep -q "no-edges-matched" "$TMPDIR/nomatch.err"

  # No predicate at all is the documented default: every edge.
  ee mechanical document new --name AllEdges --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 30 --json >/dev/null
  ee mechanical pad new --length 10 --json >/dev/null
  ee mechanical fillet new --radius 1 --json | jq -e '
    .edges_matched == 12 and .edges_length == 320
  ' >/dev/null

  # `--feature` lets a dressup name an earlier feature, not only the tip.
  # FreeCAD's own `Body::insertObject` splices it in right after that feature
  # and reroutes whatever came next to build on the dressup instead, so the
  # later feature's own contribution survives untouched.
  ee mechanical document new --name NonTipDressup --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --centered --json >/dev/null
  ee mechanical pad new --length 10 --name Base --json | jq -e '.shape.volume == 4000' >/dev/null
  ee mechanical sketch new --plane xy --offset-z 10 --json >/dev/null
  ee mechanical sketch rectangle --width 10 --height 10 --centered --json >/dev/null
  ee mechanical pad new --length 5 --name Top --json | jq -e '.shape.volume == 4500' >/dev/null

  ee mechanical fillet new --radius 1 --parallel z --feature Base --name BaseFillet --json \
    >"$TMPDIR/nontip-fillet.json"
  jq -e '
    .base == "Base" and .edges_matched == 4 and (.recompute.failed | not)
    and ((.shape.volume - 4491.415927) | fabs) < 0.001
  ' "$TMPDIR/nontip-fillet.json" >/dev/null

  # The chain, not creation order: the fillet sits between the two pads, and
  # each feature's delta is its own contribution to the running shape.
  ee mechanical document inspect --features --json | jq -e '
    [.bodies[0].features[].name] == ["Base", "BaseFillet", "Top"]
    and .bodies[0].features[0].volume_delta == 4000
    and ((.bodies[0].features[1].volume_delta - -8.584073) | fabs) < 0.001
    and .bodies[0].features[2].volume_delta == 500
  ' >/dev/null

  # More than one --feature is a refusal, not a pick of the first: a dressup's
  # Base is a single link.
  if ee mechanical fillet new --radius 1 --feature Base --feature Top --json \
      >/dev/null 2>"$TMPDIR/toomany.err"; then
    echo "fillet naming two features should have been refused" >&2
    exit 1
  fi
  grep -q "too-many-features" "$TMPDIR/toomany.err"

  # A failed dressup on a non-tip feature rolls back the whole splice, not
  # only the feature it added: the successor's rerouted link goes back too.
  before=$(ee mechanical document inspect --features --json | jq -c '[.bodies[0].features[].name]')
  if ee mechanical fillet new --radius 15 --parallel x --feature Base --json \
      >/dev/null 2>"$TMPDIR/badradius.err"; then
    echo "an oversized fillet should have been refused" >&2
    exit 1
  fi
  grep -q "recompute-failed" "$TMPDIR/badradius.err"
  after=$(ee mechanical document inspect --features --json | jq -c '[.bodies[0].features[].name]')
  test "$before" = "$after"

  # A fresh body for the chamfer, not NonTipDressup's Bar: its own "Base" is
  # already filleted on these same edges, and chamfering a filleted edge is a
  # different question than this test means to ask.
  ee mechanical document new --name NonTipChamfer --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xy --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --centered --json >/dev/null
  ee mechanical pad new --length 10 --name Base --json | jq -e '.shape.volume == 4000' >/dev/null
  ee mechanical sketch new --plane xy --offset-z 10 --json >/dev/null
  ee mechanical sketch rectangle --width 10 --height 10 --centered --json >/dev/null
  ee mechanical pad new --length 5 --name Top --json | jq -e '.shape.volume == 4500' >/dev/null

  ee mechanical chamfer new --size 1 --parallel z --feature Base --name BaseChamfer --json \
    | jq -e '.base == "Base" and .edges_matched == 4 and ((.shape.volume - 4480) | fabs) < 0.001' \
    >/dev/null

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  # Booleans between bodies: `body union`/`cut`/`intersect` fold one or more
  # tool bodies into a base body's own PartDesign chain. The tool body is not
  # deleted - only reparented - so its name and feature history stay
  # addressable, which is why `inspect --tree` still lists it, marked
  # `consumed_by` the boolean, while `inspect`'s `solids` list collapses to one.
  socket boolean

  ee mechanical document new --name Boolean --json >/dev/null
  ee mechanical body new --name A --json >/dev/null
  ee mechanical sketch new --plane xy --body A --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body A --name PadA --json | jq -e '.shape.volume == 4000' \
    >/dev/null

  ee mechanical body new --name B --json >/dev/null
  ee mechanical sketch new --plane xy --offset-x 10 --body B --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body B --json | jq -e '.shape.volume == 4000' >/dev/null

  # Two live bodies is a dead end until they are joined.
  if ee mechanical preview export --path "$TMPDIR/ambiguous.stl" --json \
      >/dev/null 2>"$TMPDIR/ambiguous.err"; then
    echo "exporting two live bodies should have been refused" >&2
    exit 1
  fi
  grep -q "ambiguous-shape" "$TMPDIR/ambiguous.err"
  ee mechanical document inspect --json | jq -e '.solids | length == 2' >/dev/null

  # Without --base and two bodies to choose from, the verb refuses rather than
  # guessing, the same discipline --body and --document already hold to.
  if ee mechanical body union --tool B --json >/dev/null 2>"$TMPDIR/noBase.err"; then
    echo "body union without --base and two bodies should have been refused" >&2
    exit 1
  fi
  grep -q "ambiguous-body" "$TMPDIR/noBase.err"

  if ee mechanical body union --tool A --base A --json >/dev/null 2>"$TMPDIR/self.err"; then
    echo "a body union of A into itself should have been refused" >&2
    exit 1
  fi
  grep -q "self-boolean" "$TMPDIR/self.err"

  # The two boxes overlap by 10x20x10 = 2000: union is 4000+4000-2000, cut is
  # A minus the overlap, intersect is the overlap alone.
  ee mechanical body union --tool B --base A --name Union --json >"$TMPDIR/union.json"
  jq -e '
    .boolean == "Union" and .operation == "union" and .tool == ["B"] and .body == "A"
    and (.recompute.failed | not) and .solid and ((.shape.volume - 6000) | fabs) < 0.001
  ' "$TMPDIR/union.json" >/dev/null

  ee mechanical document inspect --json | jq -e '.solids | length == 1' >/dev/null
  ee mechanical document inspect --tree --json | jq -e '
    (.bodies | length == 2)
    and (.bodies[] | select(.body == "A") | .consumed_by == null)
    and (.bodies[] | select(.body == "B") | .consumed_by == "Union")
    and (.bodies[0].features[] | select(.name == "Union") | .kind == "boolean"
      and .operation == "union" and .base == "PadA" and .tool == ["B"])
  ' >/dev/null

  ee mechanical document new --name BooleanCut --json >/dev/null
  ee mechanical body new --name A --json >/dev/null
  ee mechanical sketch new --plane xy --body A --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body A --json >/dev/null
  ee mechanical body new --name B --json >/dev/null
  ee mechanical sketch new --plane xy --offset-x 10 --body B --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body B --json >/dev/null
  ee mechanical body cut --tool B --base A --json | jq -e '
    .operation == "cut" and (.recompute.failed | not) and ((.shape.volume - 2000) | fabs) < 0.001
  ' >/dev/null

  ee mechanical document new --name BooleanIntersect --json >/dev/null
  ee mechanical body new --name A --json >/dev/null
  ee mechanical sketch new --plane xy --body A --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body A --json >/dev/null
  ee mechanical body new --name B --json >/dev/null
  ee mechanical sketch new --plane xy --offset-x 10 --body B --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body B --json >/dev/null
  ee mechanical body intersect --tool B --base A --json | jq -e '
    .operation == "intersect" and (.recompute.failed | not)
    and ((.shape.volume - 2000) | fabs) < 0.001
  ' >/dev/null

  # A disjoint intersect is a real case, not a bug: it has no solid at all, and
  # FreeCAD itself does not flag that as a recompute failure - so this needs
  # its own refusal, and the document must come back exactly as it was.
  ee mechanical document new --name BooleanDisjoint --json >/dev/null
  ee mechanical body new --name A --json >/dev/null
  ee mechanical sketch new --plane xy --body A --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body A --json >/dev/null
  ee mechanical body new --name B --json >/dev/null
  ee mechanical sketch new --plane xy --offset-x 100 --body B --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --json >/dev/null
  ee mechanical pad new --length 10 --body B --json >/dev/null

  before=$(ee mechanical document inspect --json | jq -c '[.objects[].name]')
  if ee mechanical body intersect --tool B --base A --json \
      >/dev/null 2>"$TMPDIR/disjoint.err"; then
    echo "an intersect of disjoint bodies should have been refused" >&2
    exit 1
  fi
  grep -q "empty-result" "$TMPDIR/disjoint.err"
  after=$(ee mechanical document inspect --json | jq -c '[.objects[].name]')
  test "$before" = "$after"
  ee mechanical document inspect --json | jq -e '.solids | length == 2' >/dev/null

  # A fuse that only touches at a single point (or an exactly tangential
  # surface) is accepted by FreeCAD's own recompute and adds the right volume,
  # but leaves a non-manifold solid every later operation returns Null shape
  # on. A revolved cone whose zero-radius apex just grazes a cube's face is
  # the smallest such case: apex at the origin, base swept out along +y, cube
  # filling the -y half-space with its top face passing through the origin.
  ee mechanical document new --name PointContact --json >/dev/null
  ee mechanical body new --name Cone --json >/dev/null
  ee mechanical sketch new --plane xy --body Cone --json >/dev/null
  ee mechanical sketch line --x1 0 --y1 0 --x2 8 --y2 20 --json >/dev/null
  ee mechanical sketch line --x1 8 --y1 20 --x2 0 --y2 20 --json >/dev/null
  ee mechanical sketch line --x1 0 --y1 20 --x2 0 --y2 0 --json | jq -e '.dof == 0' >/dev/null
  ee mechanical revolve new --axis y --angle 360 --body Cone --name ConeFeat --json \
    | jq -e '((.shape.volume - 1340.412866) | fabs) < 0.001' >/dev/null

  ee mechanical body new --name Cube --json >/dev/null
  ee mechanical sketch new --plane xz --body Cube --json >/dev/null
  ee mechanical sketch rectangle --width 20 --height 20 --centered --json >/dev/null
  ee mechanical pad new --length 20 --body Cube --name CubePad --json \
    | jq -e '.shape.volume == 8000' >/dev/null

  before=$(ee mechanical document inspect --json | jq -c '[.objects[].name]')
  if ee mechanical body union --tool Cone --base Cube --name PointFuse --json \
      >/dev/null 2>"$TMPDIR/pointContact.err"; then
    echo "a fuse touching at a single point should have been refused" >&2
    exit 1
  fi
  grep -q "degenerate-contact" "$TMPDIR/pointContact.err"
  after=$(ee mechanical document inspect --json | jq -c '[.objects[].name]')
  test "$before" = "$after"
  ee mechanical document inspect --json | jq -e '.solids | length == 2' >/dev/null

  ee mechanical session stop --force --json | jq -e '.stopped' >/dev/null

  touch $out
''

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

  ee mechanical document save --path "$document" --json | jq -e --arg f "$document" '.file == $f' >/dev/null
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

  touch $out
''

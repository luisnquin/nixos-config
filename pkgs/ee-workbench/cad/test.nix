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
    and (.constraints | length) == 11
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
    and .length == 6
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
  ee mechanical pad length --length 11 --json | jq -e '.previous == 6 and .bounds.z == 11' >/dev/null
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
    and ([.objects[].type] | index("App::Plane")) != null
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
        and ($s.constraints | length) == 11
        and ([$s.constraints[] | select(.type == "DistanceX") | .value]) == [40]
        and ([$s.constraints[] | select(.type == "DistanceY") | .value]) == [25]
        and ([$s.constraints[] | select(.type == "Coincident")] | length) == 5
        and ([$s.constraints[] | select(.type == "Horizontal")] | length) == 2
        and ([$s.constraints[] | select(.type == "Vertical")] | length) == 2
        and $s.dof == 0
        and $s.fully_constrained)
  ' "$TMPDIR/inspect.json" >/dev/null

  # Placement, named dimensions and the closed vocabulary, in one part: a
  # centred bar, a hole cut from a sketch that sits on the face above it.
  socket placed

  ee mechanical document new --name Cross --json >/dev/null
  ee mechanical body new --name Bar --json >/dev/null
  ee mechanical sketch new --plane xz --name Side --offset-z 10 --json >"$TMPDIR/side.json"
  jq -e '
    .basis.normal == {x: 0, y: -1, z: 0}
    and .basis.origin == {x: 0, y: -10, z: 0}
  ' "$TMPDIR/side.json" >/dev/null

  ee mechanical sketch new --plane xy --name Plan --json >/dev/null
  ee mechanical sketch rectangle --width 40 --height 20 --centered \
    --name-width bar_x --name-height bar_y --json >"$TMPDIR/centered.json"
  jq -e '
    .sketch == "Plan"
    and .centered
    and (.geometry | length) == 5
    and ([.geometry[] | select(.construction)] | length) == 1
    and (.constraints | length) == 12
    and ([.constraints[] | select(.type == "Symmetric")] | length) == 1
    and .dof == 0
  ' "$TMPDIR/centered.json" >/dev/null

  ee mechanical pad new --length 6 --json | jq -e '
    .shape.min == {x: -20, y: -10, z: 0} and .shape.centre_of_mass == {x: 0, y: 0, z: 3}
  ' >/dev/null

  # A name that only held until someone drove it would be worse than none: the
  # bar has to stay centred after the width changes.
  ee mechanical param list --json | jq -e '
    ([.parameters[].name] | sort) == ["bar_x", "bar_y"]
    and ([.parameters[] | select(.name == "bar_x") | .value]) == [40]
  ' >/dev/null
  ee mechanical param set bar_x 50 --json | jq -e '.value == 50 and .previous == 40' >/dev/null
  ee mechanical document recompute --json | jq -e '.failed | not' >/dev/null
  ee mechanical document inspect --json | jq -e '
    .bbox.min == {x: -25, y: -10, z: 0} and .bbox.max == {x: 25, y: 10, z: 6}
  ' >/dev/null

  # An unnamed sketch means the newest one, so this circle lands on the face.
  ee mechanical sketch new --plane xy --name Hole --offset-z 6 --json >/dev/null
  ee mechanical sketch circle --radius 4 --x 12 --json | jq -e '
    .sketch == "Hole" and .dof == 0 and (.constraints | length) == 3
  ' >/dev/null

  ee mechanical pocket new --length 3 --json >"$TMPDIR/pocket.json"
  jq -e '
    .pocket == "Pocket"
    and .sketch == "Hole"
    and .solid
    and .shape.max == {x: 25, y: 10, z: 6}
    and ((.shape.volume - (6000 - 150.796447)) | fabs) < 0.01
  ' "$TMPDIR/pocket.json" >/dev/null

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

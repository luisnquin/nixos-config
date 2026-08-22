# The proof that the slice works against real FreeCAD: it drives `ee` against a
# live `ee-freecad-server`, then reopens the saved file in a second server
# process and checks the geometry, dimensions and constraints came back.
{
  runCommand,
  jq,
  unzip,
  ee-workbench,
}:
runCommand "ee-freecad-slice-test" {
  nativeBuildInputs = [ee-workbench ee-workbench.cad jq unzip];
} ''
  set -euo pipefail

  export HOME=$TMPDIR/home
  export XDG_RUNTIME_DIR=$TMPDIR/run
  export EE_WORKBENCH_CAD_SOCKET=$TMPDIR/run/cad.sock
  mkdir -p "$HOME" "$XDG_RUNTIME_DIR"

  document=$TMPDIR/plate.FCStd

  start_session() {
    ee-freecad-server >"$TMPDIR/server-$1.log" 2>&1 &
    session_pid=$!

    for _ in $(seq 1 100); do
      if ee mechanical status --json | jq -e '.running' >/dev/null; then
        return 0
      fi
      sleep 0.2
    done

    echo "the session never came up" >&2
    cat "$TMPDIR/server-$1.log" >&2
    exit 1
  }

  stop_session() {
    kill "$session_pid"
    wait "$session_pid" || true
  }

  start_session build

  ee mechanical status --json | jq -e '.session.freecad.version | test("^[0-9]")' >/dev/null
  ee mechanical document new --name Plate --json | jq -e '.name == "Plate"' >/dev/null
  ee mechanical body new --json | jq -e '.body == "Body"' >/dev/null
  ee mechanical sketch new --plane xy --json | jq -e '.plane == "XY_Plane"' >/dev/null

  ee mechanical sketch rectangle --width 40 --height 25 --json >"$TMPDIR/rectangle.json"
  jq -e '
    (.geometry | length) == 4
    and (.constraints | length) == 11
    and .dof == 0
    and .fully_constrained
    and (.redundant | not)
    and ([.geometry[].length] | sort) == [25, 25, 40, 40]
  ' "$TMPDIR/rectangle.json" >/dev/null

  ee mechanical document recompute --json | jq -e '.failed | not' >/dev/null
  ee mechanical document save --path "$document" --json | jq -e --arg f "$document" '.file == $f' >/dev/null

  stop_session

  test -f "$document"
  unzip -l "$document" | grep -q Document.xml

  # A second process proves the geometry lives in the file, not in the session.
  start_session reopen

  ee mechanical document open "$document" --json >/dev/null
  ee mechanical document inspect --json >"$TMPDIR/inspect.json"

  stop_session

  jq -e '
    ([.objects[].type] | index("PartDesign::Body")) != null
    and ([.objects[].type] | index("App::Plane")) != null
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

  touch $out
''

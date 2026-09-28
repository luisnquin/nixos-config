# Phone

`phone` drives every Android handset, emulator and iOS simulator reachable from
this machine, including the ones on remote Macs (rose) over ssh. Use it instead
of `adb`, `emulator`, `simctl`, `avdmanager` or `ssh <host> …`: it boots with a
memory check, holds the device for your project, and survives macOS shell quirks
a hand-rolled `nohup emulator &` does not.

This page is enough to work without `phone --help`. Reach for
`phone help <verb>` only when a flag below is not enough.

## Keep calls few

A call costs about a second; the tool round trip costs more. So:

- In a repo with `phone.toml`, pass no target: its `default` applies. Elsewhere,
  `export PHONE_TARGET=<name>` from `phone device list`, in this shell only,
  never in a shell rc file.
- Put steps known in advance in one `phone do "…" "…"`: one survey, one tool
  call. A step is parsed like a command line, so quote multi-word names
  inside it: `"wait 'Order history'"`.
- An act (`tap`, `press`, `swipe`, `key`, `type`, `fill`), alone or as the
  last step of a `do`, waits for the screen to settle and prints what changed:
  `changed N new, M gone`, the new rows with an `@index` you can use, then
  what went; or `unchanged`. Do not follow it with `snapshot` or `wait` to
  check. `wait` is for something slower than the settle, like a network
  result still loading.
- Never `sleep` between an act and a read. `wait <what>` and `shot --settle`
  return as soon as the screen catches up.

## Pick a device

`phone device list` prints a per-host summary (memory left, what may still boot),
then the devices ranked by how often this project drives them. Take the top
row unless told otherwise.

| state | next step |
| --- | --- |
| `attached`, `online` | drive it |
| `off` | `phone device boot <name>` |
| `known`, `offline` | `phone device connect <name>` |
| `unauthorized` | the user accepts the dialog on the device |

A name matches on text, model, host or alias: `-t pixel_7`, `-t rose` (its most-driven free
running device), `-t rose/emu`, `-t rose/sim`.

Exit status 3 means "not now", not broken: the device is held by another project
or session, or its host has no memory for a boot. The message names what to do,
usually reusing a device that is already running. Do not pass `--take` or
`--over-budget` without asking the user.

In a repo with `phone.toml`, run `phone up` first. It boots, forwards ports,
installs a fresh build and opens the app, doing only what is missing, and is
safe to repeat. It returns once the app itself is on screen, or fails with the
dev client's error. `phone status` checks without changing anything: exit 2 is
drift on your device, 4 only on other declared ones.

## Verbs

```
phone snapshot                      # elements on screen, as text with @index
phone tap "Log in" | @3 | X,Y       # whole name, snapshot row, or coordinate
phone press <what> --hold 2s        # long press
phone swipe up|down|left|right [--amount 0.6] | swipe <from> <to> [--hold 1500ms]
phone type "text"                   # into whatever has focus
phone fill <field> "text"           # focus, clear, type, read back
phone key back|home|enter|tab|…     # go back with `key back`, not a named arrow; none on iOS
phone wait <what> [--gone] [--timeout 15s]
phone shot -o /tmp/s.png [--crop <what>|@N [--expand 1]] [--scale 0.3 --jpeg 60] [--settle] [--grid]
phone size                          # panel size and scale
phone do "fill Email a@b.co" "tap 'Log in'"  # the last act prints what changed
phone record -s 5 --frames changed
phone device list|boot|shutdown|connect|reverse 8081
phone device net off|on [--only wifi|data]   # off then on resets stale sockets
phone app install app.apk | stop <id> | open <url> | logs <id> | notifications [<id>]
phone app launch <id> [--extra KEY=VALUE]    # restarts the app with string extras
```

`-t <device>` and `--focus X,Y` work on any verb; with `do` they go on `do`, not inside a step.

## Keep reads cheap

- `shot` without `-o` goes to the clipboard, which you cannot read: always pass
  `-o <file>` and open that file.
- `snapshot` is text and usually answers the question. A full `shot` costs about
  1500 tokens; `--crop` one element, or `--scale 0.3 --jpeg 60`, costs a fraction.
- An `@index` is refused once its element moved, so take a new snapshot after
  the screen changes rather than guessing. Rows an act printed keep the
  numbers it gave them, alongside the last snapshot's.
- Rows shown as `<View>` or `<EditText>` have no name. Use their `@index`.
- A label inside a row that already reads it is not listed (the gaps in the
  `@` numbers), and a nameless pressable around one label is listed by it. Tap
  that name: it outlives the screen change that makes an `@index` stale.
- Tap by name or `@index`. When only a coordinate works, take it from the
  labels `shot --grid` draws: the image you see is scaled, so a position
  estimated off it misses.
- `unchanged` means no row moved within 3s: the act hit nothing, or its result
  is slow (network, low memory). Never repeat the act on that alone; `wait`
  for the result or check with `shot --grid`.
- `wait` passes at once when its target is already on screen, and says so.
  Wait for something the action creates, not for something already there.

## What phone does not cover

- iOS taps and bounds are in points and screenshots in pixels. A coordinate read
  off an image is divided by the `scale` from `phone size`. `--crop` already
  converts.
- In split screen, a snapshot shows both halves but keys go to the focused
  one. `--focus X,Y` presses a point first. A coordinate `tap` always goes where it points.
- Creating a new AVD or simulator: `avdmanager` or `xcrun simctl create` on its
  host, by hand. `device boot` only starts one that exists.
- Folding a foldable:
  `adb shell cmd device_state state 0|2` (closed or open), then always
  `adb shell cmd device_state state reset`.
- A physical iPhone can only be screenshotted and have its logs read.

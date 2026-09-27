# Phone

`phone` drives every Android handset, emulator and iOS simulator reachable from
this machine, including the ones on remote Macs (rose) over ssh. Use it instead
of `adb`, `emulator`, `simctl`, `avdmanager` or `ssh <host> …`: it boots with a
memory check, holds the device for your project, and survives macOS shell quirks
a hand-rolled `nohup emulator &` does not.

This page is enough to work without `phone --help`. Reach for
`phone help <verb>` only when a flag below is not enough.

## Every call costs ~8 s

Each invocation surveys all hosts before acting. So:

- Run `phone device list` once, then `export PHONE_TARGET=<name>`. Every later
  call skips the choice.
- Put steps known in advance in one `phone do "…" "…"`: one survey, one tool
  call.
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
safe to repeat. `phone status` checks without changing anything.

## Verbs

```
phone snapshot                      # elements on screen, as text with @index
phone tap "Log in" | @3 | X,Y       # name, snapshot row, or coordinate
phone press <what> --hold 2s        # long press
phone swipe up|down|left|right [--amount 0.6] | swipe <from> <to> [--hold 1500ms]
phone type "text"                   # into whatever has focus
phone fill <field> "text"           # focus, clear, type, read back
phone key back|home|enter|tab|…
phone wait <what> [--gone] [--timeout 15s]
phone shot -o /tmp/s.png [--crop <what>|@N [--expand 1]] [--scale 0.3 --jpeg 60] [--settle]
phone size                          # panel size and scale
phone do "tap 'Log in'" "wait Inbox" "shot --settle --crop Inbox"
phone record -s 5 --frames changed
phone device list|boot|shutdown|connect|reverse 8081
phone app install app.apk | launch <id> | stop <id> | open <url> | logs <id>
```

`-t <device>` and `--focus X,Y` work on any verb; with `do` they go on `do`, not inside a step.

## Keep reads cheap

- `shot` without `-o` goes to the clipboard, which you cannot read: always pass
  `-o <file>` and open that file.
- `snapshot` is text and usually answers the question. A full `shot` costs about
  1500 tokens; `--crop` one element, or `--scale 0.3 --jpeg 60`, costs a fraction.
- An `@index` is refused once its element moved, so take a new snapshot after
  the screen changes rather than guessing.
- Rows shown as `<View>` or `<EditText>` have no name. Use their `@index`.

## What phone does not cover

- iOS taps and bounds are in points and screenshots in pixels. A coordinate read
  off an image is divided by the `scale` from `phone size`. `--crop` already
  converts.
- In split screen, snapshots and keys go to the focused half. `--focus X,Y`
  presses a point first. A coordinate `tap` always goes where it points.
- Creating a new AVD or simulator: `avdmanager` or `xcrun simctl create` on its
  host, by hand. `device boot` only starts one that exists.
- Folding a foldable:
  `adb shell cmd device_state state 0|2` (closed or open), then always
  `adb shell cmd device_state state reset`.
- A physical iPhone can only be screenshotted and have its logs read.

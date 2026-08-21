# Phone

Drive the Android handsets and emulators and the iOS simulators on this desk
with the `phone` CLI: pick a device, see what is on its screen, press it, type
into it.

`phone` reaches Android over adb, directly or through an ssh host running its
own adb server. A simulator has no such transport — CoreSimulator only runs on
the Mac that owns it — so those verbs run as a `phone` on that host and come
back over ssh. Either way the commands below are the same. A physical iPhone
shows up in `phone devices` but can only be screenshotted and tailed, not read
or pressed.

**Every verb documents itself: `phone help <verb>` carries its flags, worked
examples and what it is for.** This file covers only what the CLI cannot tell
you about the devices themselves.

The verbs, in the order `phone --help` lists them:

| | |
| --- | --- |
| device | `devices` `connect` `disconnect` `pair` `pin` `use` `forget` `hosts` `boot` `shutdown` |
| screen | `snapshot` `shot` `size` `tap` `press` `swipe` `type` `key` `wait` `do` |
| app | `install` `launch` `stop` `open` `logs` |
| host | `reverse` `mirror` `record` |
| this | `doctor` `completions` |

## Pick a device

```
phone devices              # everything reachable and remembered
phone use faraday          # the default for every later command
phone connect faraday      # bring the transport up if it is not attached
```

`-t <name>` targets one command without changing the default, and `PHONE_TARGET`
does the same for a whole shell.

The state column says what to do next:

| state | meaning | next step |
| --- | --- | --- |
| `attached`, `online` | ready to drive | nothing |
| `off` | defined on its host, not running | `phone boot` |
| `known` | remembered, not visible anywhere right now | `phone connect` |
| `offline` | last seen at an address that no longer answers | `phone connect` |
| `unauthorized` | plugged in, waiting on the dialog | accept it on the device |

## The loop

A screen cannot be acted on until it has been read: `snapshot` to get names,
press one of those names, `wait` for the result, read again. Never `sleep` — a
screenshot taken right after a tap returns the frame that was already up, and
`wait` returns the moment the answer is yes.

When the steps are known in advance put them in one `do`, which surveys once and
runs them all against the same device:

```
phone do "tap 'Log in'" "wait Inbox" "shot --crop @2"
```

`snapshot` is text and usually answers the question; a full screenshot costs
roughly 1500 tokens to read, so `shot --crop <element>` when only one control
matters. What costs more than any single read is reading twice to learn whether
the first act landed — that is what `wait` and `shot --settle` are for.

Two things about names:

- A row printed as `<View>` or `<EditText>` has no name of its own. That is its
  class, in angle brackets because no verb will match on it; reach it by the
  `@index` beside it.
- `--crop <name>` finds the element carrying the name, which for a card is its
  label rather than the card. `--crop <name> --expand 1` widens to the box
  around it, `--expand 2` to the box around that.

## Coordinate spaces

Android reports element bounds, taps and screenshots all in pixels. A simulator
reports bounds and taps in **points** while screenshotting at 2x or 3x, so a
coordinate read off an iOS screenshot is not a coordinate that can be tapped.
`phone` converts internally — `--crop @N` is right on both — but a coordinate
worked out by eye from an image has to be divided by the `scale` that
`phone size` reports.

## Focus, and why a snapshot can describe the wrong app

`uiautomator` and every keyevent go to the window holding focus. In split screen
that is whichever half was touched last, so a snapshot taken while someone uses
the other half describes *that* app, and `phone key back` lands in it.

`--focus X,Y` presses a point first, in the same device-side shell as the command
that follows, so nothing interleaves:

```
phone --focus 297,1971 snapshot
```

It still loses to someone actively tapping the other half. A coordinate tap is
routed by position rather than by focus, so `phone shot` to look and
`phone tap X,Y` to act works when nothing else does.

## Foldables

Two panels, and Android keeps two unrelated id namespaces for them: a 64-bit
SurfaceFlinger id that `screencap` takes, and a small logical id that `input`
takes. `phone` reads both out of `dumpsys display` and follows whichever is
live, so folding and unfolding mid-session needs no flag.

To fold from the CLI, for a test:

```
adb shell cmd device_state print-states
adb shell cmd device_state state 0     # 0 closed, 2 opened
adb shell cmd device_state state reset # hand it back to the hinge sensor
```

Always `reset` afterwards, or the sensor stays overridden.

## Reaching a dev server from the device

`localhost` inside an emulator is the emulator, so a bundler is not reachable
until the device has a port that answers to it:

```
phone reverse 8081
phone reverse --list
phone reverse --clear
```

The port it reaches is on **the machine running the adb server that holds the
device**, not on this one. For an emulator on a mac, `phone reverse 8081` sends
the device to that mac's loopback — a Metro started over ssh there. A bundler
running here is not what it will find. Android only; a simulator is already on
its host's loopback.

## Watching something happen

A screenshot cannot show an animation, a splash that never clears, or which
list a swipe actually scrolled. `record` can, and `--frames` turns the clip into
stills that can be read:

```
phone record --seconds 8 --frames 4 --scale 0.4 --jpeg 60
phone record --seconds 8 --frames changed
```

Both halves are printed as paths. Android and simulators, up to 180 seconds.
The length is `--seconds`; the positional argument is the device, as everywhere
else.

`--frames N` spaces the stills evenly, which samples a clip blind: a route
change is short and bunched, so four even stills over five seconds are one of
the old screen and three of the same settled new one. `--frames changed` cuts at
the moments the picture actually moved instead, always keeping the first and
last frame so the before and after are anchored, and capping the count so a
scrolling list cannot return sixty stills. It says how many changes it found,
how many it kept, and where the longest stretch nothing was captured in starts
— read that line before opening any image, since it says which one to open.

A simulator encodes a frame only when the screen changes, so a clip of a screen
that sat still holds fewer frames than were asked for and says so. A still that
cannot be taken is reported and skipped: the clip and every other still are
still written, and the run still exits 0. A non-zero exit means the clip failed
or that no still could be taken at all.

## Not covered yet

No `uninstall`, no pasteboard, no `erase`, no `assert`, and no creating an AVD
or a simulator that does not exist yet — `boot` only starts one that is already
defined. Those still mean `adb`/`simctl`/`avdmanager` over ssh by hand.

## Acting blind

A coordinate tap on a screen nobody has looked at will hit whatever is there,
and app state changes are not undone by a second tap. Shoot, read the image,
then tap. If a press turns out to have opened or toggled something, say so and
put it back.

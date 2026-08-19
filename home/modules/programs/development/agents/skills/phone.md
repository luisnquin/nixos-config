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

Every invocation surveys the hosts first, which costs seconds whatever the verb
is. When the steps are known in advance put them in one `do`, which pays that
once:

```
phone do "tap 'Log in'" "wait Inbox" "shot --crop @2"
```

`snapshot` is text and usually answers the question; a full screenshot costs
roughly 1500 tokens to read, so `shot --crop <element>` when only one control
matters.

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

## Not covered yet

No screen recording, no app lifecycle (`launch`, `stop`, `open <url>`), no port
forwarding, and no creating an AVD or a simulator that does not exist yet —
`boot` only starts one that is already defined. Those still mean
`adb`/`simctl`/`avdmanager` over ssh by hand.

## Acting blind

A coordinate tap on a screen nobody has looked at will hit whatever is there,
and app state changes are not undone by a second tap. Shoot, read the image,
then tap. If a press turns out to have opened or toggled something, say so and
put it back.

# Phone

Drive the Android handsets and emulators and the iOS simulators on this desk
with the `phone` CLI: pick a device, see what is on its screen, press it, type
into it.

`phone` reaches Android over adb, directly or through an ssh host running its
own adb server. A simulator has no such transport — CoreSimulator only runs on
the Mac that owns it — so those verbs run as a `phone` on that host and come
back over ssh. Either way the commands are the same. A physical iPhone shows up
in `phone devices` but can only be screenshotted and tailed, not read or
pressed.

**The CLI documents itself. `phone --help` carries the verb list and the loop
they are meant to be used in; `phone help <verb>` carries that verb's flags,
worked examples and what it is for.** This file covers only what the CLI cannot
tell you about the devices themselves.

## Device states

`phone devices` prints a state per device, and the state says what to do next:

| state | meaning | next step |
| --- | --- | --- |
| `attached`, `online` | ready to drive | nothing |
| `off` | defined on its host, not running | `phone boot` |
| `known` | remembered, not visible anywhere right now | `phone connect` |
| `offline` | last seen at an address that no longer answers | `phone connect` |
| `unauthorized` | plugged in, waiting on the dialog | accept it on the device |

`boot` starts a device that already exists. Creating an AVD or a simulator that
was never defined is `avdmanager` or `simctl` over ssh by hand.

## Never sleep between an act and a read

A screenshot taken right after a tap returns the frame that was already up. The
device has not repainted yet and nothing in the transport waits for it, so the
image is of the screen being left rather than the one being opened — which reads
as the tap having missed.

A fixed sleep either guesses short and reads that stale frame anyway, or guesses
long and pays for it on every step. `wait <name>` returns the moment the answer
is yes and `shot --settle` returns once the screen has stopped moving. Between
them there is no case left that a sleep answers.

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

`phone` wraps none of the state overrides, so folding from the CLI for a test is
adb by hand:

```
adb shell cmd device_state print-states
adb shell cmd device_state state 0     # 0 closed, 2 opened
adb shell cmd device_state state reset # hand it back to the hinge sensor
```

Always `reset` afterwards, or the sensor stays overridden.

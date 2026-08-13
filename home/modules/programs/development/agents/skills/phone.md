# Phone

Drive the Android handsets and emulators on this desk with the `phone` CLI:
pick a device, see what is on its screen, press it, type into it.

`phone` reaches devices over adb, directly or through an ssh host that has its
own adb server. iPhones and iOS simulators appear in `phone devices` but cannot
be read or pressed.

## Pick a device

```
phone devices              # everything reachable and remembered
phone use faraday          # the default for every later command
phone connect faraday      # bring the transport up if it is not attached
```

Every command also takes `--target <name>` for a one-off. A device shown as
`known` or `offline` needs `connect` first; `attached` is ready.

## See the screen

Two ways, and they answer different questions.

```
phone shot                 # PNG to the clipboard
phone shot -o /tmp/s.png   # or to a file
phone snapshot             # the elements that can be named, one per line
phone snapshot --json
```

`snapshot` gives text, descriptions and `@index` handles that `tap` accepts. It
is empty on anything drawn rather than laid out — a game, a canvas, a video —
and there `shot` plus a coordinate is the only way through.

Do not call `adb exec-out screencap -p` by hand on a foldable. With both panels
powered it writes `[Warning] Multiple displays were found...` into stdout, ahead
of the PNG, and the file is not an image. `phone shot` passes the display id and
is unaffected.

## Press and type

```
phone tap "Log in"         # by text, description or resource id
phone tap @62              # by the index snapshot printed
phone tap 540,1200         # by coordinate
phone type "rust lang"
phone key enter            # back, home, tab, volume_up, app_switch...
```

`tap` refuses an ambiguous name and lists the candidates as `@index` rather than
guessing. `type` carries printable ASCII only and refuses the rest outright,
because the device drops what it cannot spell and still reports success.

## Focus, and why a snapshot can describe the wrong app

`uiautomator` and every keyevent go to the window holding focus. In split screen
that is whichever half was touched last, so a snapshot taken while someone uses
the other half describes that app, and `phone key back` lands in it.

`--focus X,Y` presses a point first, in the same device-side shell as the command
that follows, so nothing interleaves:

```
phone --focus 297,1971 snapshot
```

It still loses to someone actively tapping the other half. A coordinate tap is
routed by position rather than focus, so `phone shot` to look and `phone tap X,Y`
to act works when nothing else does.

## Foldables

Two panels, and Android keeps two unrelated id namespaces for them: a 64-bit
SurfaceFlinger id that `screencap` takes, and a small logical id that `input`
takes. `phone` reads both from `dumpsys display` and follows the live one, so
folding and unfolding mid-session needs no flag.

To fold from the CLI, for a test:

```
adb shell cmd device_state print-states
adb shell cmd device_state state 0     # 0 closed, 2 opened
adb shell cmd device_state state reset # hand it back to the hinge sensor
```

Always `reset` afterwards, or the sensor stays overridden.

## Acting blind

A coordinate tap on a screen you have not looked at will hit whatever is there,
and app state changes are not undone by a second tap. Shoot, read the image,
then tap. If a press turns out to have opened or toggled something, say so and
put it back.

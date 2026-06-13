# Grab the iPhone connected to the mac (rose) and put the PNG in the clipboard.
#
# iOS 17+ only exposes the screenshot service over a RemoteXPC tunnel, so rose
# runs `pymobiledevice3 remote tunneld` as a root launchd daemon. Here we just
# drive the unprivileged client over ssh and stream the PNG back into wl-copy.
#
# pymobiledevice3 exits 0 even on some failures, so success is decided by
# whether a non-empty PNG actually came back, not by the remote exit code.
copy_ios_screenshot() {
  local tmp
  tmp="$(mktemp --suffix=.png)" || return 1

  if ssh rose 'd="$(mktemp -d)"; f="$d/shot.png"; pymobiledevice3 developer dvt screenshot "$f" >/dev/null 2>&1 && cat "$f"; rm -rf "$d"' >"$tmp" && [ -s "$tmp" ]; then
    wl-copy --type image/png <"$tmp"
    print -r -- "iOS screenshot copied to clipboard"
  else
    print -ru2 -- "copy_ios_screenshot: capture failed (is the iOS device connected and unlocked?)"
    rm -f "$tmp"
    return 1
  fi

  rm -f "$tmp"
}
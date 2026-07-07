# Grab a screenshot off a connected Android device/emulator into the clipboard.
#
# adb refuses to act when more than one target is attached ("more than one
# device/emulator"), so this picks one: a bare call lists every device in the
# `device` state and, when there is more than one, prompts with fzf; an optional
# argument pre-selects by (partial) serial or model and skips the prompt.
# offline/unauthorized targets are dropped since they cannot serve a capture.
#
# Capture goes through `exec-out` rather than `shell` so the raw PNG bytes skip
# the pty's LF->CRLF translation that corrupts binary output. screencap can warn
# and still exit 0, so success is decided by a valid PNG signature coming back,
# not by adb's exit code.
copy_android_screenshot() {
  local want="$1" tmp serial sel r
  local -a rows

  rows=("${(@f)$(adb devices -l 2>/dev/null | awk '
    NR>1 && $2=="device" {
      serial=$1; model=""; product="";
      for (i=3;i<=NF;i++){ split($i,a,":");
        if(a[1]=="model") model=a[2]; if(a[1]=="product") product=a[2] }
      desc=(model!="")?model:serial;
      if(product!="") desc=desc" ("product")";
      printf "%s\t%s\n", serial, desc
    }')}")

  if [ ${#rows[@]} -eq 0 ]; then
    print -ru2 -- "copy_android_screenshot: no device online (check: adb devices)"
    return 1
  fi

  if [ -n "$want" ]; then
    for r in "${rows[@]}"; do
      case "$r" in (*"$want"*) serial="${r%%$'\t'*}"; break;; esac
    done
    if [ -z "$serial" ]; then
      print -ru2 -- "copy_android_screenshot: no online device matching '$want'"
      return 1
    fi
  elif [ ${#rows[@]} -eq 1 ]; then
    serial="${rows[1]%%$'\t'*}"
  else
    sel="$(printf '%s\n' "${rows[@]}" | fzf --delimiter='\t' --with-nth=2.. \
      --prompt='device> ' --header='select android target' --height=40% --reverse)" || return 1
    [ -n "$sel" ] || return 1
    serial="${sel%%$'\t'*}"
  fi

  tmp="$(mktemp --suffix=.png)" || return 1
  adb -s "$serial" exec-out screencap -p >"$tmp" 2>/dev/null

  if [ -s "$tmp" ] && [ "$(od -An -tx1 -N4 "$tmp" | tr -d ' \n')" = "89504e47" ]; then
    wl-copy --type image/png <"$tmp"
    rm -f "$tmp"
    print -ru2 -- "copy_android_screenshot: copied from $serial"
  else
    print -ru2 -- "copy_android_screenshot: capture failed on $serial (is it awake and unlocked?)"
    rm -f "$tmp"
    return 1
  fi
}

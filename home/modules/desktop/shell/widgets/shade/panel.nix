{
  name,
  lib,
  pkgs,
  eww,
  ...
}: let
  hark = lib.getExe pkgs.hark;

  # eww has no way to write one key of a JSON variable, so the flip round-trips
  # through jq. Both collapse maps are keyed by a slug hark emits, never by an
  # app's own name, so nothing here has to be quoted defensively.
  harkFlip = pkgs.writeShellApplication {
    name = "eww-hark-flip";
    runtimeInputs = [pkgs.jq];
    text = ''
      variable="$1"
      key="$2"
      value="$3"
      current=$(${eww} get "$variable")
      next=$(jq -c --arg k "$key" --argjson v "$value" '.[$k] = $v' <<<"$current")
      ${eww} update "$variable=$next"
    '';
  };

  flip = lib.getExe harkFlip;

  # eww 0.6 exposes no time magic variable, so the almanac heading comes off a
  # poll like every other datum in the shell.
  shadeDate = pkgs.writeShellApplication {
    name = "eww-shade-date";
    runtimeInputs = [pkgs.coreutils];
    text = ''
      date '+{"weekday":"%A","full":"%B %-d, %Y"}'
    '';
  };
in {
  yuck = ''
    ; Which groups and bodies the reader has folded open or shut. A key that is
    ; absent falls through to what hark thought was sensible for its size.
    (defvar hark_open "{}")
    (defvar hark_bodies "{}")

    (deflisten hark
      :initial '{"daemon":true,"dnd":false,"empty":true,"total":0,"active":0,"unread":0,"critical":0,"unread_critical":0,"headline":"loading","none":[],"void_rows":[],"groups":[],"panel_height":220}'
      `${hark} centre --watch`)

    (defpoll shade_date :interval "60s"
      :initial '{"weekday":"","full":""}'
      `${lib.getExe shadeDate}`)

    ; With a "top center" anchor eww reads :x as where the window's own centre
    ; lands, so 50% is dead centre whatever the shade ends up measuring.
    (defwindow ${name}
      :monitor 0
      :geometry (geometry
        :x "50%"
        :y "35px"
        :width "740px"
        :anchor "top center")
      :stacking "overlay"
      :focusable false
      (shade-widget))

    ; One root widget holding the whole tree, the way every other panel here is
    ; built. Nested argument-less defwidgets do not re-render when a global they
    ; read changes, so a footer split out into its own widget kept painting the
    ; do-not-disturb switch in whatever state it was first drawn in.
    (defwidget shade-widget []
      (box :class "shade-box" :orientation "h" :space-evenly false :spacing 16
        (box :class "shade-centre" :orientation "v" :space-evenly false :spacing 10
             :hexpand true :valign "start"
          ; The viewport has to be given a pixel height rather than grow with its
          ; contents, so hark measures the collapsed layout it is handing over
          ; and the shade is only ever as tall as it needs to be. The floor is
          ; the almanac column: below it the calendar would be the thing setting
          ; the height, and the list would sit in a short well beside it.
          (scroll :class "hark-scroll" :vscroll true
                  :height {hark.panel_height > 260 ? hark.panel_height : 260}
            (box :orientation "v" :space-evenly false :spacing 8
              (for row in {hark.empty ? hark.none : hark.groups}
                (hark-group :g row
                  :expanded {hark_open?.[row.key] ?: row.open_by_default}))
              (for line in {hark.void_rows}
                (box :class "hark-void" :orientation "v" :space-evenly false :spacing 6
                  (label :class "hark-void-mark" :text "󰇰")
                  (label :class "hark-void-text" :text line)))))

          ; Reading the centre is what clears it, so there is no "mark read"
          ; button beside this one.
          (box :class "shade-footer" :orientation "h" :space-evenly false :spacing 8
            (button :class "shade-clear" :hexpand true :halign "end"
              :tooltip "clear the centre"
              :onclick "${hark} clear"
              (label :text "Clear"))))

        (box :class "shade-divider" :vexpand true)

        (box :class "shade-almanac" :orientation "v" :space-evenly false :spacing 8
             :valign "start" :width 290
          (box :class "shade-heading" :orientation "v" :space-evenly false
            (label :class "shade-weekday" :halign "start" :text {shade_date.weekday})
            (label :class "shade-full-date" :halign "start" :text {shade_date.full}))
          (calendar :class "calendar" :show-week-numbers false))))

    (defwidget hark-group [g expanded]
      (box :class "hark-group" :orientation "v" :space-evenly false :spacing 4
        (box :class "hark-group-head" :orientation "h" :space-evenly false :spacing 8
          (button :class "hark-group-toggle" :hexpand true
            :onclick "${flip} hark_open ''${g.key} ''${expanded ? "false" : "true"}"
            (box :orientation "h" :space-evenly false :spacing 8
              (label :class "hark-chevron" :text {expanded ? "󰅀" : "󰅂"})
              (label :class {"hark-group-icon" + (g.has_critical ? " alert" : "")} :text {g.icon})
              (label :class "hark-group-label" :halign "start" :hexpand true
                :limit-width 26 :text {g.label})
              (label :class {"hark-badge" + (g.has_unread ? " unread" : "")} :text {g.count_label})
              (label :class "hark-group-when" :text {g.when})))
          (button :class "hark-tool small danger"
            :tooltip "clear this group"
            :onclick "${hark} clear ''${g.key}"
            (label :text "󰅖")))

        (for line in {expanded ? hark.none : g.preview_rows}
          (label :class "hark-group-preview" :halign "start" :limit-width 52 :text line))

        (for entry in {expanded ? g.entries : hark.none}
          (hark-entry :e entry
            :open {hark_bodies?.[entry.flip_key] ?: false}))))

    (defwidget hark-entry [e open]
      (box :class {"hark-entry " + e.urgency + (e.active ? " live" : "") + (e.unread ? " unread" : "")}
           :orientation "v" :space-evenly false :spacing 3
        (box :orientation "h" :space-evenly false :spacing 8
          (label :class "hark-entry-summary" :halign "start" :hexpand true
            :limit-width 40 :text {e.summary})
          (label :class "hark-entry-when" :halign "end" :text {e.when})
          (button :class "hark-tool small" :tooltip "remove"
            :onclick "${hark} drop --id ''${e.id}"
            (label :text "󰅖")))

        (for line in {open ? hark.none : e.rows_collapsed}
          (label :class "hark-entry-body" :halign "start" :limit-width 52 :text line))
        (for line in {open ? e.rows_expanded : hark.none}
          (label :class "hark-entry-body open" :halign "start" :wrap true :text line))

        (for value in {e.progress_rows}
          (progress :class "hark-progress" :orientation "h" :value value))

        (box :class "hark-entry-foot" :orientation "h" :space-evenly false :spacing 6
          (label :class "hark-entry-app" :halign "start" :hexpand true
            :limit-width 22 :text {e.app + " · " + e.clock})
          (for _ in {e.expand_rows}
            (button :class "hark-link"
              :onclick "${flip} hark_bodies ''${e.flip_key} ''${open ? "false" : "true"}"
              (label :text {open ? "− less" : "+ more"})))
          (for _ in {e.restore_rows}
            (button :class "hark-link" :tooltip "put it back on screen"
              :onclick "${hark} restore --id ''${e.id}"
              (label :text "󰑓 replay")))
          (for action in {e.actions}
            (button :class "hark-link action"
              :onclick "${hark} invoke --id ''${e.id} --action ''${action.key}"
              (label :limit-width 16 :text {action.label}))))))
  '';
}

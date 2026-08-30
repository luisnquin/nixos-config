{
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
in {
  yuck = ''
    ; Which groups and bodies the reader has folded open or shut. A key that is
    ; absent falls through to what hark thought was sensible for its size.
    (defvar hark_open "{}")
    (defvar hark_bodies "{}")

    (deflisten hark
      :initial '{"daemon":true,"dnd":false,"empty":true,"total":0,"active":0,"unread":0,"critical":0,"unread_critical":0,"headline":"loading","none":[],"void_rows":[],"groups":[],"panel_height":220}'
      `${hark} centre --watch`)

    (defwindow notifications
      :monitor 0
      :geometry (geometry
        :x "20px"
        :y "35px"
        :width "430px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (hark-widget))

    (defwidget hark-widget []
      (box :class "hark-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "hark-header" :orientation "h" :space-evenly false :spacing 10
          (label :class {"hark-mark" + (hark.unread_critical > 0 ? " alert" : "")}
            :text {hark.dnd ? "󰂛" : "󰇮"})
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "hark-title" :halign "start" :text "Notifications")
            (label :class "hark-meta" :halign "start" :text {hark.headline}))
          (button :class {"hark-tool" + (hark.dnd ? " on" : "")}
            :tooltip {hark.dnd ? "resume notifications" : "do not disturb"}
            :onclick "${hark} dnd toggle"
            (label :text {hark.dnd ? "󰂛" : "󰂚"}))
          (button :class "hark-tool"
            :tooltip "mark everything read"
            :onclick "${hark} seen"
            (label :text "󰗠"))
          (button :class "hark-tool danger"
            :tooltip "clear the centre"
            :onclick "${hark} clear"
            (label :text "󰩹")))

        ; The viewport has to be given a pixel height rather than grow with its
        ; contents, so hark measures the collapsed layout it is handing over and
        ; the panel is only ever as tall as it needs to be.
        (scroll :class "hark-scroll" :vscroll true :height {hark.panel_height}
          (box :orientation "v" :space-evenly false :spacing 8
            (for row in {hark.empty ? hark.none : hark.groups}
              (hark-group :g row
                :expanded {hark_open?.[row.key] ?: row.open_by_default}))
            (for line in {hark.void_rows}
              (box :class "hark-void" :orientation "v" :space-evenly false :spacing 6
                (label :class "hark-void-mark" :text "󰇰")
                (label :class "hark-void-text" :text line)))))))

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

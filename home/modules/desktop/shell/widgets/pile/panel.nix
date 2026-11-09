{
  name,
  lib,
  pkgs,
  eww,
  ...
}: let
  pile = lib.getExe pkgs.pile;
in {
  yuck = ''
    (defvar pile_open "")

    (defpoll pile :interval "1s"
      :initial '{"empty":true,"count":0,"height":22,"entries":[]}'
      `${pile} panel`)

    (defwindow ${name}
      :monitor 0
      :geometry (geometry
        :x "20px"
        :y "35px"
        :width "430px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (pile-widget))

    (defwidget pile-widget []
      (box :class "pile-box" :orientation "v" :space-evenly false :spacing 6
        (box :class "pile-head" :orientation "h" :space-evenly false
          (label :class "pile-meta" :halign "start" :hexpand true
            :text {pile.empty ? "no logs" : pile.count + " logs"})
          (button :class "pile-tool danger" :tooltip "clear"
            :onclick "${pile} clear"
            (label :text "󰩹")))

        (scroll :class "pile-scroll" :vscroll true
          :height {pile.height + (pile_open == "" ? 0 : 240)}
          (box :orientation "v" :space-evenly false :spacing 1
            (for e in {pile.entries}
              (box :class {"pile-entry " + e.urgency + (pile_open == e.key ? " open" : "")}
                :orientation "v" :space-evenly false
                (box :orientation "h" :space-evenly false
                  (button :class "pile-row" :hexpand true
                    :onclick {"${eww} update pile_open=" + (pile_open == e.key ? "" : e.key)}
                    (label :halign "start" :limit-width 52 :text {e.clock + "  " + e.line}))
                  (button :class "pile-tool small" :tooltip "remove"
                    :onclick "${pile} drop ''${e.key}"
                    (label :text "󰅖")))
                (revealer :reveal {pile_open == e.key} :transition "slidedown" :duration "120ms"
                  (box :class "pile-detail" :orientation "v" :space-evenly false :spacing 6
                    (label :class "pile-body" :halign "start" :xalign 0 :wrap true
                      :visible {e.body != ""} :text {e.body})
                    (for img in {e.images}
                      (image :halign "start" :path img :image-width 380 :preserve-aspect-ratio true))))))))))
  '';
}

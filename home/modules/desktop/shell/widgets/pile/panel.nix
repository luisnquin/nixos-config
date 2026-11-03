{
  name,
  lib,
  pkgs,
  ...
}: let
  pile = lib.getExe pkgs.pile;
in {
  yuck = ''
    (defpoll pile :interval "1s"
      :initial '{"empty":true,"count":0,"height":64,"entries":[]}'
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
      (box :class "pile-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "pile-header" :orientation "h" :space-evenly false :spacing 10
          (label :class "pile-mark" :text "󰇮")
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "pile-title" :halign "start" :text "Pile")
            (label :class "pile-meta" :halign "start"
              :text {pile.empty ? "nothing piled up" : pile.count + " kept off screen"}))
          (button :class "pile-tool danger" :tooltip "clear the pile"
            :onclick "${pile} clear"
            (label :text "󰩹")))

        (scroll :class "pile-scroll" :vscroll true :height {pile.height}
          (box :orientation "v" :space-evenly false :spacing 8
            (for e in {pile.entries}
              (box :class {"pile-entry " + e.urgency} :orientation "v" :space-evenly false :spacing 3
                (box :orientation "h" :space-evenly false :spacing 8
                  (label :class "pile-entry-summary" :halign "start" :hexpand true
                    :limit-width 40 :text {e.summary})
                  (label :class "pile-entry-when" :halign "end" :text {e.clock})
                  (button :class "pile-tool small" :tooltip "remove"
                    :onclick "${pile} drop ''${e.key}"
                    (label :text "󰅖")))
                (label :class "pile-entry-body" :halign "start" :wrap true :text {e.body})
                (label :class "pile-entry-app" :halign "start" :text {e.app})))))))
  '';
}

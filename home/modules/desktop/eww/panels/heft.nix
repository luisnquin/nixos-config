{
  lib,
  pkgs,
  ...
}: let
  # `heft eww` is a cache read, never a scan — the timer owns collection.
  heftInfo = pkgs.writeShellApplication {
    name = "eww-heft";
    runtimeInputs = [pkgs.heft];
    text = ''
      heft eww
    '';
  };

  # The panel copies an action by index rather than interpolating the command
  # into a shell line: the commands contain paths, `&&` and quotes, and none of
  # that should ever be re-parsed on the way to the clipboard.
  heftCopy = pkgs.writeShellApplication {
    name = "eww-heft-copy";
    runtimeInputs = with pkgs; [heft jq wl-clipboard];
    text = ''
      heft eww | jq -j --argjson i "''${1:?index required}" '.actions[$i].command // ""' | wl-copy
    '';
  };
in {
  yuck = ''
    (defpoll heft_data :interval "5m"
      :initial '{"filesystem":{"total":0,"used":0,"free":0},"pct_used":0,"domains":[],"movers":[],"reclaimable":0,"actions":[],"newcomers":[],"updated_label":"loading"}'
      `${lib.getExe heftInfo}`)

    (defwindow heft
      :monitor 0
      :geometry (geometry
        :x "235px"
        :y "35px"
        :width "430px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (heft-widget))

    (defwidget heft-domain [domain]
      (box :orientation "v" :space-evenly false :spacing 2
        (box :orientation "h" :space-evenly false
          (label :class "heft-domain-name" :halign "start" :hexpand true :text {domain.name})
          (label :class "heft-domain-size" :halign "end"
            :text "''${round(domain.bytes / 1073741824, 1)}G"))
        (progress :class "heft-bar" :orientation "h" :value {domain.pct})))

    (defwidget heft-action [action]
      (button :class {"heft-action " + action.safety}
        :tooltip "click to copy the command"
        :onclick "${lib.getExe heftCopy} ''${action.rank}"
        (box :orientation "v" :space-evenly false
          (box :orientation "h" :space-evenly false
            (label :class "heft-action-label" :halign "start" :hexpand true :limit-width 38 :text {action.label})
            (label :class "heft-action-frees" :halign "end"
              :text "''${round(action.frees / 1073741824, 1)}G"))
          (label :class "heft-action-command" :halign "start" :limit-width 52 :text {action.command}))))

    (defwidget heft-widget []
      (box :class "heft-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "heft-header" :orientation "h" :space-evenly false :spacing 10
          (label :class "heft-mark" :text "󰋊")
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "heft-title" :halign "start"
              :text "''${round(heft_data.filesystem.free / 1073741824, 1)}G free")
            (label :class "heft-meta" :halign "start"
              :text "''${round(heft_data.pct_used, 0)}% of ''${round(heft_data.filesystem.total / 1073741824, 0)}G used · ''${round(heft_data.reclaimable / 1073741824, 1)}G reclaimable")))

        ; Every data-dependent section lives in here. The layer-shell surface is
        ; sized once, from `:initial`, and does not renegotiate when the poll
        ; lands — so the panel's natural height has to be the same whether the
        ; lists are empty or full, or the sections get stacked on top of one
        ; another in an allocation that never grew.
        ; `:visible` is deliberately absent throughout: eww shows a toggled-back
        ; box without its parent renegotiating the allocation, so it paints over
        ; its siblings. Empty sections say so in their title instead.
        (scroll :class "heft-scroll" :vscroll true :height 620
          (box :orientation "v" :space-evenly false :spacing 12
            (box :class "heft-section" :orientation "v" :space-evenly false :spacing 6
              (label :class "heft-section-title" :halign "start" :text "WHERE IT WENT")
              (for domain in {heft_data.domains}
                (heft-domain :domain {domain})))

            (box :class "heft-section" :orientation "v" :space-evenly false :spacing 4
              (label :class "heft-section-title" :halign "start"
                :text {arraylength(heft_data.movers) > 0
                  ? "MOVED SINCE LAST SCAN"
                  : "NO EARLIER SCAN TO COMPARE AGAINST"})
              (for mover in {heft_data.movers}
                (box :orientation "h" :space-evenly false
                  (label :class "heft-mover-name" :halign "start" :hexpand true :text {mover.label})
                  (label :class {mover.delta > 0 ? "heft-mover-delta grew" : "heft-mover-delta shrank"} :halign "end"
                    :text {mover.delta_label}))))

            (box :class "heft-section" :orientation "v" :space-evenly false :spacing 5
              (label :class "heft-section-title" :halign "start"
                :text {arraylength(heft_data.actions) > 0
                  ? "RECLAIM"
                  : "NOTHING WORTH RECLAIMING"})
              (for action in {heft_data.actions}
                (heft-action :action {action})))

            (box :class "heft-section" :orientation "v" :space-evenly false :spacing 4
              (label :class "heft-section-title" :halign "start"
                :text {arraylength(heft_data.newcomers) > 0
                  ? "NEW IN THE STORE"
                  : "NOTHING NEW IN THE STORE"})
              (for entry in {heft_data.newcomers}
                (box :orientation "h" :space-evenly false
                  (label :class "heft-entry-title" :halign "start" :hexpand true :limit-width 40 :text {entry.label})
                  (label :class "heft-entry-meta" :halign "end" :text {entry.bytes_label}))))))

        (label :class "heft-updated" :halign "end" :text {heft_data.updated_label})))
  '';
}

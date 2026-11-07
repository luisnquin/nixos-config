{
  name,
  config,
  lib,
  pkgs,
  eww,
  ...
}: let
  terminals = "^(ghostty|com\\.mitchellh\\.ghostty|Alacritty)";

  away = pkgs.writeShellApplication {
    name = "away";
    runtimeInputs = with pkgs; [coreutils jq libnotify systemd] ++ [config.wayland.windowManager.hyprland.package];
    text = ''
      apps() {
        hyprctl -j clients | jq -c --arg t '${terminals}' '[.[] | select(.class | test($t) | not)]'
      }

      apps | jq -r 'group_by(.pid)[] | if length == 1 then "window \(.[0].address)" else "pid \(.[0].pid)" end' |
        while read -r kind id; do
          case "$kind" in
            window) hyprctl dispatch "hl.dsp.window.close({ window = \"address:$id\" })" >/dev/null ;;
            pid) kill -TERM "$id" 2>/dev/null || true ;;
          esac
        done

      for _ in $(seq 50); do
        [ "$(apps | jq length)" -eq 0 ] && break
        sleep 0.2
      done

      left=$(apps | jq -r '[.[].class] | unique | join(", ")')
      if [ -n "$left" ]; then
        notify-send -a away "Away cancelled" "Still open: $left"
        exit 1
      fi

      systemctl --user stop tray.target hyprland-session.target graphical-session.target
      hyprctl dispatch 'hl.dsp.exit()' >/dev/null
    '';
  };

  status = pkgs.writeShellApplication {
    name = "eww-session-status";
    runtimeInputs = with pkgs; [coreutils jq systemd tmux] ++ [config.wayland.windowManager.hyprland.package];
    text = ''
      herdr=$(systemctl --user is-active herdr-hub.service || true)
      tmux=$(systemctl --user is-active tmux.service || true)
      sessions=$(TMUX_TMPDIR="$XDG_RUNTIME_DIR" tmux list-sessions 2>/dev/null | wc -l || true)
      apps=$(hyprctl -j clients | jq --arg t '${terminals}' '[.[] | select(.class | test($t) | not)] | length')

      jq -cn --arg herdr "$herdr" --arg tmux "$tmux" --argjson sessions "$sessions" --argjson apps "$apps" \
        '{herdr: ($herdr == "active"), tmux: ($tmux == "active"), sessions: $sessions, apps: $apps}'
    '';
  };

  leave = "${eww} close ${name}; systemd-run --user --collect --quiet ${lib.getExe away}";
in {
  yuck = ''
    (defpoll session :interval "3s"
      :initial '{"herdr":false,"tmux":false,"sessions":0,"apps":0}'
      `${lib.getExe status}`)

    (defwindow ${name}
      :monitor 0
      :geometry (geometry
        :x "8px"
        :y "35px"
        :width "280px"
        :anchor "top left")
      :stacking "overlay"
      :focusable false
      (session-widget))

    (defwidget session-keep [label ok detail]
      (box :class {"ses-row " + (ok ? "ok" : "bad")} :orientation "h" :space-evenly false :spacing 10
        (label :class {"ses-dot " + (ok ? "ok" : "bad")} :text {ok ? "●" : "○"})
        (label :class "ses-name" :hexpand true :halign "start" :text label)
        (label :class "ses-detail" :halign "end" :text {ok ? detail : "dies with hyprland"})))

    (defwidget session-widget []
      (box :class "ses-box" :orientation "v" :space-evenly false :spacing 10
        (box :class "ses-header" :orientation "v" :space-evenly false
          (label :class "ses-title" :halign "start" :text "Away")
          (label :class "ses-meta" :halign "start" :text "drop to tty, keep the agents"))
        (session-keep :label "herdr hub" :ok {session.herdr} :detail "kept")
        (session-keep :label "tmux" :ok {session.tmux} :detail {session.sessions + " sessions kept"})
        (label :class "ses-note" :halign "start" :wrap true
          :text {(session.apps == 0 ? "no apps open" : (session.apps == 1 ? "closes 1 window" : "closes " + session.apps + " windows")) + " · log in to come back"})
        (box :class "ses-actions" :orientation "h" :space-evenly true :spacing 8
          (button :class "ses-btn" :onclick "${eww} close ${name}" "cancel")
          (button :class "ses-btn go" :onclick "${leave}" "go to tty"))))
  '';
}

{
  config,
  lib,
  pkgs,
  eww,
  ...
}: let
  batteryInfo = pkgs.writeShellApplication {
    name = "eww-battery-info";
    runtimeInputs = with pkgs; [coreutils gawk jq];
    text = ''
      bat=""
      for path in /sys/class/power_supply/*; do
        [ -r "$path/type" ] || continue
        [ "$(cat "$path/type")" = "Battery" ] || continue
        bat="$path"
        break
      done

      if [ -z "$bat" ]; then
        jq -cn '{present:false, icon:"󰂑", state_class:"missing", status:"missing", percent:0, pct_label:"—", source:"AC", health:"—", health_label:"Health", rate:"—", voltage:"—", eta:"—", capacity_level:"—", model:"No battery"}'
        exit 0
      fi

      name="$(basename "$bat")"
      status="$(cat "$bat/status" 2>/dev/null || echo Unknown)"
      capacity="$(cat "$bat/capacity" 2>/dev/null || echo 0)"
      capacity_level="$(cat "$bat/capacity_level" 2>/dev/null || echo "—")"
      model="$(cat "$bat/model_name" 2>/dev/null || echo "$name")"
      manufacturer="$(cat "$bat/manufacturer" 2>/dev/null || true)"

      now="$(cat "$bat/energy_now" 2>/dev/null || cat "$bat/charge_now" 2>/dev/null || echo 0)"
      full="$(cat "$bat/energy_full" 2>/dev/null || cat "$bat/charge_full" 2>/dev/null || echo 0)"
      design="$(cat "$bat/energy_full_design" 2>/dev/null || cat "$bat/charge_full_design" 2>/dev/null || echo 0)"
      rate="$(cat "$bat/power_now" 2>/dev/null || cat "$bat/current_now" 2>/dev/null || echo 0)"
      voltage="$(cat "$bat/voltage_now" 2>/dev/null || echo 0)"

      ac_online=0
      for path in /sys/class/power_supply/*; do
        [ -r "$path/type" ] || continue
        type="$(cat "$path/type")"
        if [ "$type" = "Mains" ] || [ "$type" = "USB" ]; then
          online="$(cat "$path/online" 2>/dev/null || echo 0)"
          [ "$online" = "1" ] && ac_online=1
        fi
      done

      metrics="$(awk -v st="$status" -v cap="$capacity" -v now="$now" -v full="$full" -v design="$design" -v rate="$rate" -v volt="$voltage" -v ac="$ac_online" '
        function human_power(v) {
          if (v <= 0) return "idle";
          return sprintf("%.1f W", v / 1000000);
        }
        function human_volt(v) {
          if (v <= 0) return "—";
          return sprintf("%.2f V", v / 1000000);
        }
        function human_eta(hours,   total, h, m) {
          if (hours <= 0 || hours > 240) return "—";
          total = int(hours * 60 + 0.5);
          h = int(total / 60);
          m = total % 60;
          if (h > 0) return sprintf("%dh %02dm", h, m);
          return sprintf("%dm", m);
        }
        BEGIN {
          health = (design > 0 && full > 0) ? int((full / design) * 100 + 0.5) : -1;
          if (rate > 0 && st == "Discharging") eta = human_eta(now / rate);
          else if (rate > 0 && st == "Charging") eta = human_eta((full - now) / rate);
          else eta = "—";

          if (st == "Charging") icon = "󰂄";
          else if (ac == 1 && cap >= 95) icon = "󱟢";
          else if (cap < 10) icon = "󰁺";
          else if (cap < 25) icon = "󰁻";
          else if (cap < 45) icon = "󰁽";
          else if (cap < 65) icon = "󰁿";
          else if (cap < 85) icon = "󰂀";
          else icon = "󰂂";

          state = tolower(st);
          gsub(/[^a-z0-9_-]/, "-", state);
          printf "%s\t%s\t%s\t%s\t%s", icon, state, human_power(rate), human_volt(volt), eta;
          printf "\t%s", health >= 0 ? health "%" : "—";
          printf "\t%s", ac == 1 ? "AC" : "Battery";
        }')"

      icon="$(printf '%s' "$metrics" | cut -f1)"
      state_class="$(printf '%s' "$metrics" | cut -f2)"
      rate_label="$(printf '%s' "$metrics" | cut -f3)"
      voltage_label="$(printf '%s' "$metrics" | cut -f4)"
      eta="$(printf '%s' "$metrics" | cut -f5)"
      health="$(printf '%s' "$metrics" | cut -f6)"
      source="$(printf '%s' "$metrics" | cut -f7)"

      jq -cn \
        --arg icon "$icon" --arg state_class "$state_class" --arg status "$status" \
        --arg pct_label "$capacity%" --arg source "$source" --arg health "$health" \
        --arg rate "$rate_label" --arg voltage "$voltage_label" --arg eta "$eta" \
        --arg capacity_level "$capacity_level" --arg model "$model" --arg manufacturer "$manufacturer" \
        --arg name "$name" --argjson percent "$capacity" \
        '{present:true, icon:$icon, state_class:$state_class, status:$status,
          percent:$percent, pct_label:$pct_label, source:$source, health:$health,
          health_label:"Health", rate:$rate, voltage:$voltage, eta:$eta,
          capacity_level:$capacity_level, model:($manufacturer + $model), name:$name}'
    '';
  };
in {
  yuck = ''
    (defpoll bat :interval "3s"
      :initial '{"present":false,"icon":"󰂑","state_class":"missing","status":"loading","percent":0,"pct_label":"—","source":"—","health":"—","health_label":"Health","rate":"—","voltage":"—","eta":"—","capacity_level":"—","model":"Battery"}'
      `${lib.getExe batteryInfo}`)

    (defwindow battery
      :monitor 0
      :geometry (geometry
        :x "8px"
        :y "35px"
        :width "300px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (bat-widget))

    (defwidget bat-widget []
      (box :class {"bat-box " + bat.state_class} :orientation "v" :space-evenly false :spacing 12
        (box :class "bat-hero" :orientation "h" :space-evenly false :spacing 14
          (box :class "bat-gauge" :orientation "v" :space-evenly false :valign "center" :spacing 5
            (box :class "bat-icon" :orientation "h" :space-evenly false
              (progress :class "bat-icon-fill" :value {bat.percent}))
            (label :class "bat-gauge-value" :text {bat.pct_label}))
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "bat-percent" :halign "start" :text {bat.pct_label})
            (label :class "bat-status" :halign "start" :text {bat.status + " · " + bat.source})
            (label :class "bat-model" :halign "start" :limit-width 28 :text {bat.model})))
        (box :class "bat-grid" :orientation "v" :space-evenly false :spacing 4
          (bat-row :label "Time" :value {bat.eta})
          (bat-row :label "Draw" :value {bat.rate})
          (bat-row :label "Voltage" :value {bat.voltage})
          (bat-row :label {bat.health_label} :value {bat.health})
          (bat-row :label "Level" :value {bat.capacity_level}))
        (progress :class "bat-bar" :value {bat.percent})
        (box :class "bat-actions" :orientation "h" :space-evenly true :spacing 8
          (button :class "bat-btn" :onclick "${eww} poll bat" "refresh")
          (button :class "bat-btn" :onclick "${eww} close battery & ${lib.getExe config.programs.ghostty.package} --class=waybar.btop -e ${pkgs.btop}/bin/btop" "btop"))))

    (defwidget bat-row [label value]
      (box :class "bat-row" :orientation "h" :space-evenly false
        (label :class "bat-row-label" :halign "start" :hexpand true :text label)
        (label :class "bat-row-value" :halign "end" :text value)))
  '';
}

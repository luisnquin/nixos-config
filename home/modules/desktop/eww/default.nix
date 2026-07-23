{
  config,
  lib,
  pkgs,
  ...
}: let
  eww = lib.getExe config.programs.eww.package;

  nmtuiCmd = "${lib.getExe config.programs.ghostty.package} --class=eww.nmtui -e ${pkgs.networkmanager}/bin/nmtui connect";

  scanCmd = "${lib.getExe pkgs.scripts.nmcli-wifi-scan-waybar} --scan";

  systemInfo = pkgs.writeShellApplication {
    name = "eww-system-info";
    runtimeInputs = with pkgs; [coreutils gawk jq procps];
    text = ''
      state="/tmp/eww-system-info.state"

      read -r _ user nice system idle iowait irq softirq steal guest guest_nice < /proc/stat
      total=$((user + nice + system + idle + iowait + irq + softirq + steal + guest + guest_nice))
      idle_all=$((idle + iowait))

      prev_total=0
      prev_idle=0
      if [ -r "$state" ]; then
        read -r prev_total prev_idle < "$state" || true
      fi
      printf '%s %s\n' "$total" "$idle_all" > "$state"

      cpu_percent="$(awk -v total="$total" -v idle="$idle_all" -v ptotal="$prev_total" -v pidle="$prev_idle" '
        BEGIN {
          dt = total - ptotal;
          di = idle - pidle;
          if (dt <= 0) print 0;
          else printf "%d", ((dt - di) / dt) * 100 + 0.5;
        }')"

      cores="$(nproc)"
      load="$(cut -d' ' -f1-3 /proc/loadavg)"
      uptime_label="$(awk '{d=int($1 / 86400); h=int(($1 % 86400) / 3600); m=int(($1 % 3600) / 60); if (d > 0) printf "%dd %02dh", d, h; else printf "%dh %02dm", h, m}' /proc/uptime)"
      freq_label="$(awk -F': ' '/cpu MHz/ {sum += $2; n++} END {if (n > 0) printf "%.2f GHz", (sum / n) / 1000; else print "—"}' /proc/cpuinfo)"
      temp_label="$(awk '
        function valid(path, value) {
          getline value < path;
          close(path);
          return value != "" && value > 0;
        }
        BEGIN {
          best = "";
          for (i = 0; i < 32; i++) {
            type = "/sys/class/thermal/thermal_zone" i "/type";
            temp = "/sys/class/thermal/thermal_zone" i "/temp";
            if ((getline name < type) > 0) {
              close(type);
              if (name ~ /x86_pkg_temp|k10temp|cpu|CPU|package|Package/) {
                if (valid(temp, raw)) { best = raw; break; }
              }
            }
          }
          if (best == "") {
            for (i = 0; i < 32; i++) {
              temp = "/sys/class/thermal/thermal_zone" i "/temp";
              if (valid(temp, raw)) { best = raw; break; }
            }
          }
          if (best == "") print "—";
          else printf "%.0f°C", best / 1000;
        }')"

      mem="$(awk '
        function human(kb,   v, unit) {
          v = kb / 1024 / 1024;
          unit = "GiB";
          return sprintf("%.1f %s", v, unit);
        }
        /^MemTotal:/ {total = $2}
        /^MemAvailable:/ {avail = $2}
        /^SwapTotal:/ {swap_total = $2}
        /^SwapFree:/ {swap_free = $2}
        END {
          used = total - avail;
          pct = total > 0 ? int((used / total) * 100 + 0.5) : 0;
          swap_used = swap_total - swap_free;
          swap_pct = swap_total > 0 ? int((swap_used / swap_total) * 100 + 0.5) : 0;
          printf "%d\t%s\t%s\t%s\t%d", pct, human(used), human(total), human(swap_used), swap_pct;
        }' /proc/meminfo)"
      mem_percent="$(printf '%s' "$mem" | cut -f1)"
      mem_used="$(printf '%s' "$mem" | cut -f2)"
      mem_total="$(printf '%s' "$mem" | cut -f3)"
      swap_used="$(printf '%s' "$mem" | cut -f4)"
      swap_percent="$(printf '%s' "$mem" | cut -f5)"

      top_procs() {
        ps -eo "$1"=,comm= --sort=-"$1" \
          | awk 'NR <= 8 {v = $1; $1 = ""; sub(/^[[:space:]]+/, ""); if ($0 == "") next; printf "%s\t%.1f%%\n", $0, v}' \
          | jq -R -s -c 'split("\n") | map(select(length > 0) | split("\t") | {name: .[0], value: .[1]})'
      }
      top_cpu_all="$(top_procs %cpu)"
      top_mem_all="$(top_procs %mem)"
      top_cpu="$(printf '%s' "$top_cpu_all" | jq -c '.[0:3]')"
      top_mem="$(printf '%s' "$top_mem_all" | jq -c '.[0:3]')"

      jq -cn \
        --argjson cpu_percent "$cpu_percent" --arg cores "$cores" --arg load "$load" \
        --arg uptime "$uptime_label" --arg freq "$freq_label" --arg temp "$temp_label" \
        --argjson mem_percent "$mem_percent" --arg mem_used "$mem_used" --arg mem_total "$mem_total" \
        --arg swap_used "$swap_used" --argjson swap_percent "$swap_percent" \
        --argjson top_cpu "$top_cpu" --argjson top_cpu_all "$top_cpu_all" \
        --argjson top_mem "$top_mem" --argjson top_mem_all "$top_mem_all" \
        '{cpu_percent:$cpu_percent, cpu_label:($cpu_percent | tostring) + "%",
          cores:$cores, load:$load, uptime:$uptime, freq:$freq, temp:$temp,
          mem_percent:$mem_percent, mem_label:($mem_percent | tostring) + "%",
          mem_used:$mem_used, mem_total:$mem_total, swap_used:$swap_used,
          swap_percent:$swap_percent, swap_label:($swap_percent | tostring) + "%",
          top_cpu:$top_cpu, top_cpu_all:$top_cpu_all,
          top_mem:$top_mem, top_mem_all:$top_mem_all}'
    '';
  };

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

  netInfo = pkgs.writeShellApplication {
    name = "eww-network-info";
    runtimeInputs = with pkgs; [iproute2 iw coreutils gawk jq];
    text = ''
      state="/tmp/eww-network-rate.state"

      dev="$(ip route show default 2>/dev/null | awk '/^default/ {print $5; exit}')"

      if [ -z "$dev" ]; then
        jq -cn '{connected:false, icon:"󰤮", primary:"Disconnected", subtitle:"no route", signal_label:"", ip:"—", gateway:"—", freq_label:"—", up:"0 B/s", down:"0 B/s"}'
        exit 0
      fi

      rx="$(cat "/sys/class/net/$dev/statistics/rx_bytes" 2>/dev/null || echo 0)"
      tx="$(cat "/sys/class/net/$dev/statistics/tx_bytes" 2>/dev/null || echo 0)"
      now="$(date +%s%N)"

      p_dev=""; p_rx=0; p_tx=0; p_now=0
      if [ -r "$state" ]; then
        read -r p_dev p_rx p_tx p_now < "$state" || true
      fi
      printf '%s %s %s %s\n' "$dev" "$rx" "$tx" "$now" > "$state"

      rates="$(awk -v dev="$dev" -v rx="$rx" -v tx="$tx" -v now="$now" \
                   -v pdev="$p_dev" -v prx="$p_rx" -v ptx="$p_tx" -v pnow="$p_now" '
        function human(b,   u, i) {
          split("B/s kB/s MB/s GB/s", u, " ");
          i = 1; while (b >= 1024 && i < 4) { b /= 1024; i++ }
          return sprintf("%.1f %s", b, u[i]);
        }
        BEGIN {
          if (pdev != dev || pnow == 0 || now <= pnow) { up = 0; down = 0 }
          else {
            dt = (now - pnow) / 1e9;
            up = (tx - ptx) / dt; down = (rx - prx) / dt;
            if (up < 0) up = 0; if (down < 0) down = 0;
          }
          printf "%s\t%s", human(up), human(down);
        }')"
      up="$(printf '%s' "$rates" | cut -f1)"
      down="$(printf '%s' "$rates" | cut -f2)"

      ip4="$(ip -4 -o addr show dev "$dev" 2>/dev/null | awk '{print $4}' | cut -d/ -f1 | head -n1)"
      [ -z "$ip4" ] && ip4="—"
      gw="$(ip -4 route show default dev "$dev" 2>/dev/null | awk '/default/ {print $3; exit}')"
      [ -z "$gw" ] && gw="—"

      if [ -d "/sys/class/net/$dev/wireless" ]; then
        link="$(iw dev "$dev" link 2>/dev/null || true)"
        ssid="$(printf '%s\n' "$link" | awk -F': ' '/SSID/ {print $2; exit}')"
        [ -z "$ssid" ] && ssid="$dev"
        dbm="$(printf '%s\n' "$link" | awk '/signal:/ {print $2; exit}')"
        freq="$(printf '%s\n' "$link" | awk '/freq:/ {printf "%d", $2; exit}')"
        if [ -n "$freq" ]; then freq_label="$freq MHz"; else freq_label="—"; fi
        if [ -n "$dbm" ]; then
          signal_label="$dbm dBm"
          pct="$(awk -v d="$dbm" 'BEGIN { p = 2 * (d + 100); if (p < 0) p = 0; if (p > 100) p = 100; print int(p) }')"
        else
          signal_label=""; pct=0
        fi
        icon="$(awk -v p="$pct" 'BEGIN {
          if (p >= 80) print "󰤨"; else if (p >= 60) print "󰤥";
          else if (p >= 40) print "󰤢"; else if (p >= 20) print "󰤟"; else print "󰤯" }')"
        primary="$ssid"
        subtitle="$dev · Wi-Fi"
      else
        icon="󰈀"
        primary="Ethernet"
        subtitle="$dev · wired"
        signal_label=""
        freq_label="—"
      fi

      jq -cn \
        --arg icon "$icon" --arg primary "$primary" --arg subtitle "$subtitle" \
        --arg signal_label "$signal_label" --arg ip "$ip4" --arg gateway "$gw" \
        --arg freq_label "$freq_label" --arg up "$up" --arg down "$down" \
        '{connected:true, icon:$icon, primary:$primary, subtitle:$subtitle,
          signal_label:$signal_label, ip:$ip, gateway:$gateway,
          freq_label:$freq_label, up:$up, down:$down}'
    '';
  };

  githubInfo = pkgs.writeShellApplication {
    name = "eww-github-monitor";
    runtimeInputs = with pkgs; [coreutils jq];
    text = ''
      cache=${lib.escapeShellArg "${config.xdg.cacheHome}/github-monitor/state.json"}
      if [ -r "$cache" ]; then
        cat "$cache"
      else
        jq -cn '{workflows:[],issues:[],errors:[],workflow_count:0,failed_count:0,issue_count:0,updated_at:"waiting for first refresh"}'
      fi
    '';
  };

  tailscaleInfo = pkgs.writeShellApplication {
    name = "eww-tailscale-info";
    runtimeInputs = with pkgs; [coreutils jq tailscale];
    text = ''
      if ! json="$(tailscale status --json 2>/dev/null)"; then
        jq -cn '{connected:false,online_count:0,total_count:0,self_name:"—",self_ip:"—",tailnet:"—",devices:[]}'
        exit 0
      fi

      printf '%s' "$json" | jq -c '
        now as $now
        | def osmeta:
            {
              "linux":   {icon:"\uf17c", label:"Linux"},
              "macOS":   {icon:"\uf179", label:"macOS"},
              "iOS":     {icon:"\uf10b", label:"iOS"},
              "android": {icon:"\uf17b", label:"Android"},
              "windows": {icon:"\uf17a", label:"Windows"},
              "freebsd": {icon:"\uf30c", label:"FreeBSD"}
            }[.] // {icon:"\uf108", label:(if (. == null or . == "") then "unknown" else . end)};
          def rel($d):
            if $d < 60 then "just now"
            elif $d < 3600 then (($d/60|floor|tostring) + "m ago")
            elif $d < 86400 then (($d/3600|floor|tostring) + "h ago")
            elif $d < 2592000 then (($d/86400|floor|tostring) + "d ago")
            else (($d/2592000|floor|tostring) + "mo ago") end;
          def mk($isSelf):
            (.OS // "" | osmeta) as $os
            | (.Online // false) as $on
            | (.LastSeen // "" | sub("\\.[0-9]+";"") | try fromdateiso8601 catch 0) as $ls
            | {
                name: (.HostName // "device"),
                os: $os.label,
                os_icon: $os.icon,
                online: $on,
                is_self: $isSelf,
                ip: (.TailscaleIPs[0] // "—"),
                status: (if $isSelf then "this device"
                         elif $on then "online"
                         elif $ls > 0 then rel($now - $ls)
                         else "unknown" end),
                status_class: (if $on then "online" else "offline" end),
                sort_key: (if $on then $now else $ls end)
              };
          (.Self // {}) as $self
          | ([ ($self | mk(true)) ] + [ (.Peer // {} | to_entries[] | .value | mk(false)) ]) as $all
          | ($all | sort_by(.sort_key) | reverse) as $devices
          | {
              connected: true,
              online_count: ($devices | map(select(.online)) | length),
              total_count: ($devices | length),
              self_name: ($self.HostName // "—"),
              self_ip: ($self.TailscaleIPs[0] // "—"),
              tailnet: ($self.DNSName // "" | rtrimstr(".")
                        | (split(".") | if length > 1 then (.[1:] | join(".")) else "tailnet" end)),
              devices: $devices
            }'
    '';
  };
in {
  programs.eww = {
    enable = true;
    package = pkgs.eww;

    systemd = {
      enable = true;
      target = "graphical-session.target";
    };

    yuckConfig = ''
      (defwindow calendar
        :monitor 0
        :geometry (geometry
          :x "50%"
          :y "35px"
          :width "270px"
          :anchor "top center")
        :stacking "overlay"
        :focusable false
        (cal-widget))

      (defwidget cal-widget []
        (box :class "cal-box" :orientation "v" :space-evenly false
          (calendar :class "calendar"
                    :show-week-numbers false)))

      (defvar cpu_expanded false)
      (defvar mem_expanded false)

      (defpoll sys :interval "2s"
        :initial '{"cpu_percent":0,"cpu_label":"—","cores":"—","load":"—","uptime":"—","freq":"—","temp":"—","mem_percent":0,"mem_label":"—","mem_used":"—","mem_total":"—","swap_used":"—","swap_percent":0,"swap_label":"—","top_cpu":[],"top_cpu_all":[],"top_mem":[],"top_mem_all":[]}'
        `${lib.getExe systemInfo}`)

      (defwindow sysmon
        :monitor 0
        :geometry (geometry
          :x "100px"
          :y "35px"
          :width "300px"
          :anchor "top right")
        :stacking "overlay"
        :focusable false
        (sysmon-widget))

      (defwidget sysmon-widget []
        (box :class "sys-box" :orientation "v" :space-evenly false :spacing 14
          (cpu-section)
          (box :class "sys-divider")
          (mem-section)))

      (defwidget cpu-section []
        (box :class "cpu-box" :orientation "v" :space-evenly false :spacing 12
          (box :class "sys-hero" :orientation "h" :space-evenly false :spacing 14
            (box :class "sys-meter cpu-meter" :orientation "v" :space-evenly false :valign "center"
              (label :class "sys-meter-icon" :halign "center" :text "󰍛")
              (label :class "sys-meter-value" :halign "center" :text {sys.cpu_label}))
            (box :orientation "v" :space-evenly false :hexpand true :halign "start"
              (label :class "sys-title" :halign "start" :text "CPU")
              (label :class "sys-subtitle" :halign "start" :text {sys.cores + " cores · " + sys.freq})
              (progress :class "sys-bar cpu-bar" :value {sys.cpu_percent})))
          (box :class "sys-grid" :orientation "v" :space-evenly false :spacing 4
            (sys-row :label "Load" :value {sys.load})
            (sys-row :label "Temp" :value {sys.temp})
            (sys-row :label "Uptime" :value {sys.uptime})
            (top-list :items {sys.top_cpu} :all {sys.top_cpu_all}
                      :expanded {cpu_expanded} :toggle "cpu_expanded"))))

      (defwidget mem-section []
        (box :class "mem-box" :orientation "v" :space-evenly false :spacing 12
          (box :class "sys-hero" :orientation "h" :space-evenly false :spacing 14
            (box :class "sys-meter mem-meter" :orientation "v" :space-evenly false :valign "center"
              (label :class "sys-meter-icon" :halign "center" :text "")
              (label :class "sys-meter-value" :halign "center" :text {sys.mem_label}))
            (box :orientation "v" :space-evenly false :hexpand true :halign "start"
              (label :class "sys-title" :halign "start" :text "Memory")
              (label :class "sys-subtitle" :halign "start" :text {sys.mem_used + " / " + sys.mem_total})
              (progress :class "sys-bar mem-bar" :value {sys.mem_percent})))
          (box :class "sys-grid" :orientation "v" :space-evenly false :spacing 4
            (sys-row :label "Used" :value {sys.mem_used})
            (sys-row :label "Total" :value {sys.mem_total})
            (sys-row :label "Swap" :value {sys.swap_used + " · " + sys.swap_label})
            (top-list :items {sys.top_mem} :all {sys.top_mem_all}
                      :expanded {mem_expanded} :toggle "mem_expanded"))))

      (defwidget sys-row [label value]
        (box :class "sys-row" :orientation "h" :space-evenly false
          (label :class "sys-row-label" :halign "start" :hexpand true :text label)
          (label :class "sys-row-value" :halign "end" :text value)))

      (defwidget top-list [items all expanded toggle]
        (box :class "top-list" :orientation "v" :space-evenly false :spacing 4
          (box :class "top-head" :orientation "h" :space-evenly false
            (label :class "top-head-label" :halign "start" :hexpand true :text "Top processes")
            (button :class "top-toggle"
              :onclick {"${eww} update " + toggle + "=" + (expanded ? "false" : "true")}
              (label :text {expanded ? "− less" : "+ more"})))
          (for p in {expanded ? all : items}
            (proc-row :name {p.name} :value {p.value}))))

      (defwidget proc-row [name value]
        (box :class "sys-row proc-row" :orientation "h" :space-evenly false
          (label :class "sys-row-label proc-name" :halign "start" :hexpand true :limit-width 24 :text name)
          (label :class "sys-row-value proc-value" :halign "end" :text value)))

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

      (defpoll net :interval "2s"
        :initial '{"connected":false,"icon":"󰤫","primary":"…","subtitle":"","signal_label":"","ip":"—","gateway":"—","freq_label":"—","up":"0 B/s","down":"0 B/s"}'
        `${lib.getExe netInfo}`)

      (defwindow network
        :monitor 0
        :geometry (geometry
          :x "50px"
          :y "35px"
          :width "260px"
          :anchor "top right")
        :stacking "overlay"
        :focusable false
        (net-widget))

      (defwidget net-widget []
        (box :class "net-box" :orientation "v" :space-evenly false :spacing 12
          (box :class "net-header" :orientation "h" :space-evenly false :spacing 10
            (label :class "net-icon" :text {net.icon})
            (box :orientation "v" :space-evenly false :hexpand true :halign "start"
              (label :class "net-ssid" :halign "start" :limit-width 18 :text {net.primary})
              (label :class "net-type" :halign "start" :text {net.subtitle}))
            (label :class "net-dbm" :valign "start" :text {net.signal_label}))
          (box :orientation "v" :space-evenly false :spacing 4
            (net-row :label "IP" :value {net.ip})
            (net-row :label "Gateway" :value {net.gateway})
            (net-row :label "Frequency" :value {net.freq_label}))
          (box :class "net-rates" :orientation "h" :space-evenly true
            (label :class "net-up" :text {"↑ " + net.up})
            (label :class "net-down" :text {"↓ " + net.down}))
          (box :class "net-actions" :orientation "h" :space-evenly true :spacing 8
            (button :class "net-btn" :onclick "${eww} close network & ${nmtuiCmd}" "nmtui")
            (button :class "net-btn" :onclick "${scanCmd}" "force scan"))))

      (defwidget net-row [label value]
        (box :class "net-row" :orientation "h" :space-evenly false
          (label :class "net-row-label" :halign "start" :hexpand true :text label)
          (label :class "net-row-value" :halign "end" :text value)))

      (defpoll github_data :interval "30s"
        :initial '{"workflows":[],"issues":[],"errors":[],"workflow_count":0,"failed_count":0,"issue_count":0,"updated_at":"loading"}'
        `${lib.getExe githubInfo}`)

      (defwindow github
        :monitor 0
        :geometry (geometry
          :x "235px"
          :y "35px"
          :width "390px"
          :anchor "top right")
        :stacking "overlay"
        :focusable false
        (github-widget))

      (defwidget github-widget []
        (box :class "github-box" :orientation "v" :space-evenly false :spacing 12
          (box :class "github-header" :orientation "h" :space-evenly false :spacing 10
            (label :class "github-mark" :text "")
            (box :orientation "v" :space-evenly false :hexpand true :halign "start"
              (label :class "github-title" :halign "start" :text "GitHub watch")
              (label :class "github-meta" :halign "start"
                :text {github_data.failed_count + " failed / " + github_data.workflow_count + " watched · " + github_data.issue_count + " issues"})))
          (box :class "github-section" :orientation "v" :space-evenly false :spacing 5
            (label :class "github-section-title" :halign "start" :text "WATCHED WORKFLOWS")
            (for run in {github_data.workflows}
              (button :class {"github-entry " + run.conclusion} :onclick {"${lib.getExe' pkgs.xdg-utils "xdg-open"} " + run.url}
                (box :orientation "v" :space-evenly false
                  (label :class "github-entry-title" :halign "start" :limit-width 43 :text {run.displayTitle})
                  (label :class "github-entry-meta" :halign "start" :text {run.repo + " · " + run.workflowName + " · " + run.conclusion})))))
          (box :class "github-section" :orientation "v" :space-evenly false :spacing 5
            (label :class "github-section-title" :halign "start" :text "ISSUES · NEWEST 3")
            (for issue in {github_data.issues}
              (button :class "github-entry issue" :onclick {"${lib.getExe' pkgs.xdg-utils "xdg-open"} " + issue.url}
                (box :orientation "v" :space-evenly false
                  (label :class "github-entry-title" :halign "start" :limit-width 43 :text {"#" + issue.number + "  " + issue.title})
                  (label :class "github-entry-meta" :halign "start" :text {issue.repo})))))
          (for source in {github_data.errors}
            (label :class "github-error" :halign "start" :text {"query failed · " + source}))
          (label :class "github-updated" :halign "end" :text {github_data.updated_at})))

      (defpoll ts :interval "5s"
        :initial '{"connected":false,"online_count":0,"total_count":0,"self_name":"—","self_ip":"—","tailnet":"—","devices":[]}'
        `${lib.getExe tailscaleInfo}`)

      (defwindow tailscale
        :monitor 0
        :geometry (geometry
          :x "160px"
          :y "35px"
          :width "320px"
          :anchor "top right")
        :stacking "overlay"
        :focusable false
        (ts-widget))

      (defwidget ts-widget []
        (box :class "ts-box" :orientation "v" :space-evenly false :spacing 12
          (box :class "ts-header" :orientation "h" :space-evenly false :spacing 10
            (label :class {"ts-mark " + (ts.connected ? "up" : "down")} :text "")
            (box :orientation "v" :space-evenly false :hexpand true :halign "start"
              (label :class "ts-title" :halign "start" :text "Tailscale")
              (label :class "ts-meta" :halign "start"
                :text {ts.connected
                  ? (ts.online_count + "/" + ts.total_count + " online · " + ts.tailnet)
                  : "disconnected"}))
            (label :class "ts-self-ip" :valign "start" :text {ts.self_ip}))
          (scroll :class "ts-scroll" :vscroll true :height 280
            (box :class "ts-list" :orientation "v" :space-evenly false :spacing 5
              (for d in {ts.devices}
                (box :class {"ts-entry " + d.status_class + (d.is_self ? " self" : "")}
                     :orientation "h" :space-evenly false :spacing 10
                  (label :class {"ts-os " + d.status_class} :text {d.os_icon})
                  (box :orientation "v" :space-evenly false :hexpand true :halign "start"
                    (label :class "ts-name" :halign "start" :limit-width 24 :text {d.name})
                    (label :class "ts-sub" :halign "start" :text {d.os + " · " + d.ip}))
                  (box :orientation "v" :space-evenly false :halign "end"
                    (label :class {"ts-dot " + d.status_class} :halign "end" :text {d.online ? "●" : "○"})
                    (label :class "ts-time" :halign "end" :text {d.status}))))))
          (box :class "ts-actions" :orientation "h" :space-evenly true :spacing 8
            (button :class "ts-btn" :onclick "${eww} poll ts" "refresh")
            (button :class "ts-btn"
              :onclick {"${eww} close tailscale & ${lib.getExe' pkgs.xdg-utils "xdg-open"} https://login.tailscale.com/admin/machines"}
              "admin panel"))))
    '';

    scssConfig = ''
      * {
        font-family: "IoskeleyMono Nerd Font", "Symbols Nerd Font", monospace;
        font-size: 10pt;
        font-weight: 600;
        color: #cdd6f4;
      }

      window,
      .background {
        background-color: transparent;
      }

      .cal-box {
        background-color: rgba(29, 16, 58, 0.97);
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 10px;
        padding: 12px 16px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
      }

      calendar {
        background-color: transparent;
        color: #cdd6f4;
        border-radius: 6px;
      }

      calendar.header {
        background-color: transparent;
        color: #e8cef5;
        font-weight: bold;
        padding: 2px 0 6px 0;
      }

      calendar.day-name {
        background-color: transparent;
        color: #b5e8e0;
        font-weight: bold;
        padding: 2px 0;
      }

      calendar.day {
        background-color: transparent;
        color: #cdd6f4;
        border-radius: 4px;
        padding: 1px 3px;
      }

      calendar.day:hover {
        background-color: rgba(181, 232, 224, 0.08);
      }

      calendar.other-month {
        color: rgba(205, 214, 244, 0.28);
      }

      calendar.today {
        color: #f28fad;
        font-weight: bold;
        text-decoration: underline;
      }

      calendar:selected {
        background-color: #f28fad;
        color: #1a1826;
        border-radius: 4px;
      }

      calendar button {
        background-color: transparent;
        color: #a9b1d6;
        border: none;
        box-shadow: none;
        min-width: 0;
        min-height: 0;
        padding: 2px 6px;
      }

      calendar button:hover {
        background-color: rgba(181, 232, 224, 0.08);
        color: #e8cef5;
      }

      calendar button label {
        color: inherit;
      }

      .sys-box {
        background-color: rgba(29, 16, 58, 0.97);
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 10px;
        padding: 14px 16px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
      }

      .sys-divider {
        background-color: rgba(181, 232, 224, 0.10);
        min-height: 1px;
      }

      .sys-meter {
        background-color: rgba(205, 214, 244, 0.08);
        border: 1px solid rgba(205, 214, 244, 0.12);
        border-radius: 8px;
        min-width: 64px;
        min-height: 64px;
        padding: 7px 8px;
      }

      .cpu-meter {
        background-color: rgba(245, 194, 231, 0.14);
        border-color: rgba(245, 194, 231, 0.28);
      }

      .mem-meter {
        background-color: rgba(181, 232, 224, 0.14);
        border-color: rgba(181, 232, 224, 0.28);
      }

      .sys-meter-icon {
        font-size: 18pt;
        color: #1a1826;
      }

      .cpu-meter .sys-meter-icon,
      .cpu-meter .sys-meter-value {
        color: #f5c2e7;
      }

      .cpu-meter .sys-meter-icon {
        margin-right: 4px;
      }

      .mem-meter .sys-meter-icon,
      .mem-meter .sys-meter-value {
        color: #b5e8e0;
      }

      .mem-meter .sys-meter-icon {
        margin-right: 10px;
      }

      .sys-meter-value {
        font-size: 13pt;
      }

      .sys-title {
        font-size: 18pt;
        color: #ffffff;
      }

      .sys-subtitle {
        font-size: 8pt;
        font-weight: normal;
        color: rgba(205, 214, 244, 0.50);
      }

      .sys-bar trough {
        background-color: rgba(205, 214, 244, 0.12);
        border-radius: 999px;
        min-height: 6px;
        margin-top: 8px;
      }

      .sys-bar progress {
        border-radius: 999px;
        min-height: 6px;
      }

      .cpu-bar progress {
        background-color: #f5c2e7;
      }

      .mem-bar progress {
        background-color: #b5e8e0;
      }

      .sys-grid {
        border-top: 1px solid rgba(181, 232, 224, 0.10);
        padding-top: 10px;
      }

      .sys-row-label {
        color: rgba(205, 214, 244, 0.55);
        font-weight: normal;
      }

      .sys-row-value {
        color: #cdd6f4;
      }

      .top-list {
        border-top: 1px solid rgba(181, 232, 224, 0.10);
        margin-top: 4px;
        padding-top: 8px;
      }

      .top-head {
        margin-bottom: 2px;
      }

      .top-head-label {
        color: #b5e8e0;
        font-size: 8pt;
        letter-spacing: 1px;
      }

      .top-toggle {
        background-color: rgba(181, 232, 224, 0.07);
        color: rgba(205, 214, 244, 0.75);
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 6px;
        padding: 1px 8px;
        font-size: 8pt;
        font-weight: normal;
      }

      .top-toggle:hover {
        background-color: rgba(181, 232, 224, 0.14);
        color: #e8cef5;
      }

      .proc-value {
        color: #b5e8e0;
      }

      .bat-box {
        background-color: rgba(29, 16, 58, 0.97);
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 10px;
        padding: 14px 16px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
      }

      .bat-gauge {
        background-color: rgba(181, 232, 224, 0.12);
        border: 1px solid rgba(181, 232, 224, 0.30);
        border-radius: 8px;
        min-width: 64px;
        min-height: 64px;
        padding: 7px 8px;
      }

      .bat-icon {
        background-color: transparent;
        border: 2px solid #1a1826;
        border-radius: 4px;
        min-width: 42px;
        min-height: 20px;
        padding: 2px;
      }

      .bat-icon-fill trough {
        background-color: transparent;
        border-radius: 2px;
        min-height: 16px;
        min-width: 38px;
      }

      .bat-icon-fill progress {
        background-color: #1a1826;
        border-radius: 2px;
        min-height: 16px;
      }

      .bat-gauge-value {
        font-size: 13pt;
        color: #1a1826;
      }

      .bat-percent {
        font-size: 19pt;
        color: #ffffff;
      }

      .bat-status {
        font-size: 9pt;
        color: #b5e8e0;
      }

      .bat-model {
        font-size: 8pt;
        font-weight: normal;
        color: rgba(205, 214, 244, 0.50);
      }

      .bat-grid {
        border-top: 1px solid rgba(181, 232, 224, 0.10);
        padding-top: 10px;
      }

      .bat-row-label {
        color: rgba(205, 214, 244, 0.55);
        font-weight: normal;
      }

      .bat-row-value {
        color: #cdd6f4;
      }

      .bat-bar trough {
        background-color: rgba(205, 214, 244, 0.12);
        border-radius: 999px;
        min-height: 6px;
      }

      .bat-bar progress {
        background-color: #b5e8e0;
        border-radius: 999px;
        min-height: 6px;
      }

      .bat-box.charging .bat-gauge,
      .bat-box.full .bat-gauge,
      .bat-box.charging .bat-bar progress,
      .bat-box.full .bat-bar progress {
        background-color: #b5e8e0;
      }

      .bat-box.charging .bat-gauge-value,
      .bat-box.full .bat-gauge-value {
        color: #1a1826;
      }

      .bat-box.discharging .bat-gauge,
      .bat-box.discharging .bat-bar progress {
        background-color: #f8bd96;
      }

      .bat-box.discharging .bat-gauge-value {
        color: #1a1826;
      }

      .bat-btn {
        background-color: rgba(181, 232, 224, 0.07);
        color: #cdd6f4;
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 6px;
        padding: 5px 10px;
      }

      .bat-btn:hover {
        background-color: rgba(181, 232, 224, 0.14);
        color: #e8cef5;
      }

      .net-box {
        background-color: rgba(29, 16, 58, 0.97);
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 10px;
        padding: 14px 16px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
      }

      .net-icon {
        font-size: 22pt;
        color: #b5e8e0;
        margin-right: 4px;
      }

      .net-ssid {
        font-size: 11pt;
        color: #e8cef5;
      }

      .net-type {
        font-size: 8pt;
        font-weight: normal;
        color: rgba(205, 214, 244, 0.45);
      }

      .net-dbm {
        font-size: 8pt;
        color: #b5e8e0;
      }

      .net-row-label {
        color: rgba(205, 214, 244, 0.55);
        font-weight: normal;
      }

      .net-row-value {
        color: #cdd6f4;
      }

      .net-rates {
        border-top: 1px solid rgba(181, 232, 224, 0.10);
        padding-top: 10px;
      }

      .net-up {
        color: #b5e8e0;
      }

      .net-down {
        color: #f28fad;
      }

      .net-btn {
        background-color: rgba(181, 232, 224, 0.07);
        color: #cdd6f4;
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 6px;
        padding: 5px 10px;
      }

      .net-btn:hover {
        background-color: rgba(181, 232, 224, 0.14);
        color: #e8cef5;
      }

      .github-box {
        background-color: rgba(29, 16, 58, 0.98);
        border: 1px solid rgba(181, 232, 224, 0.14);
        border-radius: 10px;
        padding: 14px 16px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
      }

      .github-header {
        border-bottom: 1px solid rgba(181, 232, 224, 0.10);
        padding-bottom: 10px;
      }

      .github-mark {
        color: #b5e8e0;
        font-size: 23pt;
      }

      .github-title {
        color: #ffffff;
        font-size: 15pt;
      }

      .github-meta,
      .github-entry-meta,
      .github-updated {
        color: rgba(205, 214, 244, 0.48);
        font-size: 8pt;
        font-weight: normal;
      }

      .github-section-title {
        color: #b5e8e0;
        font-size: 8pt;
        letter-spacing: 1px;
      }

      .github-entry {
        background-color: rgba(205, 214, 244, 0.045);
        border: 1px solid rgba(205, 214, 244, 0.08);
        border-radius: 6px;
        padding: 7px 9px;
      }

      .github-entry:hover {
        background-color: rgba(181, 232, 224, 0.10);
        border-color: rgba(181, 232, 224, 0.20);
      }

      .github-entry.failure,
      .github-entry.cancelled,
      .github-entry.timed_out,
      .github-entry.startup_failure,
      .github-entry.action_required {
        border-left: 3px solid #f28fad;
      }

      .github-entry.success {
        border-left: 3px solid #b5e8e0;
      }

      .github-entry.issue {
        border-left: 3px solid #f8bd96;
      }

      .github-entry-title {
        color: #cdd6f4;
      }

      .github-error {
        color: #f28fad;
        font-size: 8pt;
      }

      .ts-box {
        background-color: rgba(29, 16, 58, 0.98);
        border: 1px solid rgba(181, 232, 224, 0.14);
        border-radius: 10px;
        padding: 14px 16px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
      }

      .ts-header {
        border-bottom: 1px solid rgba(181, 232, 224, 0.10);
        padding-bottom: 10px;
      }

      .ts-mark {
        font-size: 22pt;
        margin-right: 2px;
      }

      .ts-mark.up {
        color: #b5e8e0;
      }

      .ts-mark.down {
        color: rgba(205, 214, 244, 0.35);
      }

      .ts-title {
        color: #ffffff;
        font-size: 15pt;
      }

      .ts-meta {
        color: rgba(205, 214, 244, 0.48);
        font-size: 8pt;
        font-weight: normal;
      }

      .ts-self-ip {
        color: #b5e8e0;
        font-size: 8pt;
      }

      .ts-list {
        margin-top: 2px;
      }

      .ts-entry {
        background-color: rgba(205, 214, 244, 0.045);
        border: 1px solid rgba(205, 214, 244, 0.08);
        border-left: 3px solid rgba(205, 214, 244, 0.15);
        border-radius: 6px;
        padding: 7px 10px;
      }

      .ts-entry.online {
        border-left: 3px solid #b5e8e0;
      }

      .ts-entry.self {
        background-color: rgba(181, 232, 224, 0.07);
      }

      .ts-os {
        font-size: 17pt;
        margin-right: 2px;
        color: rgba(205, 214, 244, 0.45);
      }

      .ts-os.online {
        color: #b5e8e0;
      }

      .ts-name {
        color: #cdd6f4;
        font-size: 10pt;
      }

      .ts-sub {
        color: rgba(205, 214, 244, 0.45);
        font-size: 8pt;
        font-weight: normal;
      }

      .ts-dot {
        font-size: 9pt;
      }

      .ts-dot.online {
        color: #b5e8e0;
      }

      .ts-dot.offline {
        color: rgba(205, 214, 244, 0.30);
      }

      .ts-time {
        color: rgba(205, 214, 244, 0.55);
        font-size: 8pt;
        font-weight: normal;
      }

      .ts-actions {
        border-top: 1px solid rgba(181, 232, 224, 0.10);
        padding-top: 10px;
      }

      .ts-btn {
        background-color: rgba(181, 232, 224, 0.07);
        color: #cdd6f4;
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 6px;
        padding: 5px 10px;
      }

      .ts-btn:hover {
        background-color: rgba(181, 232, 224, 0.14);
        color: #e8cef5;
      }
    '';
  };
}

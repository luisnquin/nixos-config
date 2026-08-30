{pkgs, ...}: {
  home.packages = [
    pkgs.libnotify
  ];

  services.kdeconnect = {
    enable = true;
    indicator = true;
  };

  services.hark = {
    enable = true;

    # Volume and brightness OSDs come off an out-of-tree script through
    # dunstify. They are a readout the popup already delivered, so the centre
    # never accounts for them.
    ignore = [
      {appName = "(?i)^dunstify$";}
    ];

    groups = {
      agents = {
        label = "Coding agents";
        icon = "󰚩";
        priority = 10;
        alwaysCollapse = true;
        # Agents reach the bus under their own name when they own the process
        # and under notify-send when a hook fires them, so both spellings have
        # to land in the same bucket.
        match = [
          {appName = "(?i)(claude|codex|gemini|opencode|aider|cursor|copilot)";}
          {
            appName = "(?i)notify-send";
            summary = "(?i)\\b(claude|codex|gemini|opencode|aider|roborev)\\b";
          }
        ];
      };

      phone = {
        label = "Phone";
        icon = "󰄜";
        priority = 20;
        match = [
          {appName = "(?i)(kdeconnect|phone)";}
          {desktopEntry = "(?i)kdeconnect";}
        ];
      };

      media = {
        label = "Media";
        icon = "󰝚";
        priority = 30;
        match = [
          {category = "^mpd$";}
          {appName = "(?i)(mpd|spotify|playerctl|mpv)";}
        ];
      };

      # libx.notify defaults the app name to the title, so host commands
      # announce themselves under their own headline rather than a stable id.
      system = {
        label = "System";
        icon = "󰒓";
        priority = 40;
        match = [
          {appName = "(?i)(battery-notifier|systemd|nixos|hark)";}
          {appName = "(?i)^nyx\\b";}
          {summary = "(?i)\\b(battery|charge|disk|thermal)\\b";}
        ];
      };
    };
  };
}

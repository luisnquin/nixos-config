{pkgs, ...}: {
  home.packages = [
    pkgs.libnotify
  ];

  services.kdeconnect = {
    enable = true;
    indicator = true;
  };

  services.pile = {
    enable = true;
    rules = [
      {app-name = "phone";}
      {app-name = ".grimblast-wrapped";}
      {app-name = "Hyprshot";}
      {app-name = "Claude Code";}
      {app-name = "Herdr";}
      {
        app-name = "notify-send";
        "summary~" = "(claude|codex|gemini|opencode|aider)";
      }
    ];
  };
}

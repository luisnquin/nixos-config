{pkgs, ...}: {
  services.ntfy-sh-client = {
    enable = true;

    options = let
      ntfy-icon = builtins.path {
        name = "ntfy-sh.png";
        path = ../dots/ntfy-sh.png;
      };
    in {
      default-command = ''${pkgs.libnotify}/bin/notify-send --icon=${ntfy-icon} "ntfy.sh/$NTFY_TOPIC" "$NTFY_MESSAGE"'';
      subscribe = [
        {
          topic = "hello";
        }
      ];
    };
  };
}

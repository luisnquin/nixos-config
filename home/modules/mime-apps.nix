{...}: {
  xdg.mimeApps = {
    enable = true;

    defaultApplications = let
      braveDesktop = "brave.desktop";
    in {
      "x-scheme-handler/unknown" = braveDesktop;
      "x-scheme-handler/https" = braveDesktop;
      "x-scheme-handler/about" = braveDesktop;
      "x-scheme-handler/http" = braveDesktop;
      "text/html" = braveDesktop;

      "x-scheme-handler/mailto" = "vivaldi-stable.desktop";
      "x-scheme-handler/slack" = "slack.desktop";
      "inode/directory" = "org.kde.dolphin.desktop";
    };
  };
}
# {
#   "brave.desktop" = [
#     "x-scheme-handler/unknown"
#     "x-scheme-handler/https"
#     "x-scheme-handler/about"
#     "x-scheme-handler/http"
#     "text/html"
#   ];
#   "kilo.desktop" = [
#     "text/csv"
#   ];
# }


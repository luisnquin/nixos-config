{
  inputs,
  system,
  ...
}: {
  home.packages = let
    zen-browser = inputs.zen-browser.packages.${system}.default;
  in [
    zen-browser
  ];

  xdg.mimeApps = let
    associations = builtins.listToAttrs (map (name: {
        inherit name;
        value = "zen.desktop";
      }) [
        "x-scheme-handler/http"
        "x-scheme-handler/https"
        "x-scheme-handler/mailto"
        "x-scheme-handler/unknown"
        "x-scheme-handler/about"
        "application/json"
        "text/plain"
        "text/html"
        "image/*"
      ]);
  in {
    associations.added = associations;
    defaultApplications = associations;
  };
}

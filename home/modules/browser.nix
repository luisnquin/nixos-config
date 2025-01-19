{
  inputs,
  system,
  pkgs,
  ...
}: let
  zen-browser = inputs.zen-browser.packages.${system}.beta;
in {
  home.packages = [
    zen-browser
  ];

  xdg.mimeApps = let
    associations = builtins.listToAttrs (map (name: {
        inherit name;
        value = zen-browser.meta.desktopFile;
      }) [
        "application/x-extension-shtml"
        "application/x-extension-xhtml"
        "application/x-extension-html"
        "application/x-extension-xht"
        "application/x-extension-htm"
        "x-scheme-handler/unknown"
        "x-scheme-handler/mailto"
        "x-scheme-handler/chrome"
        "x-scheme-handler/about"
        "x-scheme-handler/https"
        "x-scheme-handler/http"
        "application/xhtml+xml"
        "application/json"
        "text/plain"
        "text/html"
        "image/*"
      ]);
  in {
    associations.added = associations;
    defaultApplications = associations;
  };

  programs.chromium = {
    enable = true;
    package = pkgs.brave;
    extensions = let
      ids = [
        "dmghijelimhndkbmpgbldicpogfkceaj" # "Dark mode"
        "gppongmhjkpfnbhagpmjfkannfbllamg" # Wappalyzer - Technology profiler

        "dnhpnfgdlenaccegplpojghhmaamnnfp" # Augmented Steam
        "ficfmibkjjnpogdcfhfokmihanoldbfe" # File Icons for GitHub and GitLab

        "njkkfaliiinmkcckepjdmgbmjljfdeee" # CodeWing
        "ldleapnlgbkpkabhbkkeangmnfpikahe" # What's new on GitHub
        "ialbpcipalajnakfondkflpkagbkdoib" # Lovely forks
        "anlikcnbgdeidpacdbdljnabclhahhmd" # Enhanced GitHub
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

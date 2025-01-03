{
  inputs,
  system,
  pkgs,
  host,
  ...
}: let
  zen-browser = inputs.zen-browser.packages.${system}.twilight;
in {
  home.packages = [
    zen-browser
  ];

  xdg.mimeApps = let
    associations = builtins.listToAttrs (map (name: {
        inherit name;
        value = zen-browser.meta.desktopFile;
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

  programs.chromium = {
    enable = true;
    package = pkgs."${host.browser}";
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

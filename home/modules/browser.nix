{
  zen-browser,
  pkgs,
  host,
  ...
}: {
  home.packages = [
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

  programs.chromium = {
    enable = true;
    package = pkgs."${host.browser}";
    extensions = let
      ids = [
        "dmghijelimhndkbmpgbldicpogfkceaj" # "Dark mode"
        "emjjmkkcdllendgbldliiphlbpnomnjk" # target="_blank"-toggler
        "gppongmhjkpfnbhagpmjfkannfbllamg" # Wappalyzer - Technology profiler

        "dnhpnfgdlenaccegplpojghhmaamnnfp" # Augmented Steam
        "ficfmibkjjnpogdcfhfokmihanoldbfe" # File Icons for GitHub and GitLab
        "obnjfbgikjcdfnbnmdamffacjfpankih" # Excalisave

        "oldceeleldhonbafppcapldpdifcinji" # TLDR: Grammar checker
        "aeblfdkhhhdcdjpifhhbdiojplfjncoa" # 1Password
        "fihnjjcciajhdojfnbdddfaoknhalnja" # I don't care about cookies
        "njdfdhgcmkocbgbhcioffdbicglldapd" # LocalCDN
        # The precursors of "privacy" are assholes but still, I have personal reasons to use this

        "nhdogjmejiglipccpnnnanhbledajbpd" # Vue.js devtools
        "fmkadmapgofadopljbjfkapdkoienihi" # React Developer Tools
        "pfefekfhnmbfcofpjojnpmhdakcadeil" # AstroSpect

        # GitHub
        "njkkfaliiinmkcckepjdmgbmjljfdeee" # CodeWing
        "ldleapnlgbkpkabhbkkeangmnfpikahe" # What's new on GitHub
        "ialbpcipalajnakfondkflpkagbkdoib" # Lovely forks
        "anlikcnbgdeidpacdbdljnabclhahhmd" # Enhanced GitHub
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

{
  pkgs,
  host,
  ...
}: {
  home.packages = [
    pkgs.mullvad-browser
    pkgs.vivaldi # Always available
  ];

  programs.chromium = {
    enable = true;
    package = pkgs."${host.browser}";
    extensions = let
      ids = [
        "dmghijelimhndkbmpgbldicpogfkceaj" # "Dark mode"
        "emjjmkkcdllendgbldliiphlbpnomnjk" # target="_blank"-toggler
        "gppongmhjkpfnbhagpmjfkannfbllamg" # Wappalyzer - Technology profiler

        "dnhpnfgdlenaccegplpojghhmaamnnfp" # Augmented Steam
        "anlikcnbgdeidpacdbdljnabclhahhmd" # Enhanced GitHub
        "ficfmibkjjnpogdcfhfokmihanoldbfe" # File Icons for GitHub and GitLab
        "obnjfbgikjcdfnbnmdamffacjfpankih" # Excalisave

        "nhdogjmejiglipccpnnnanhbledajbpd" # Vue.js devtools
        "fmkadmapgofadopljbjfkapdkoienihi" # React Developer Tools
        "pfefekfhnmbfcofpjojnpmhdakcadeil" # AstroSpect
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

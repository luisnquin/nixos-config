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
        "iaiomicjabeggjcfkbimgmglanimpnae" # Tab manager
        "dmghijelimhndkbmpgbldicpogfkceaj" # "Dark mode"
        "nigigpmchbpkjjgncmpiggfnikllldlh" # Forever pinned
        "emjjmkkcdllendgbldliiphlbpnomnjk" # target="_blank"-toggler
        "gppongmhjkpfnbhagpmjfkannfbllamg" # Wappalyzer - Technology profiler
        "nikomkkhhpfoeamojhhgpfkpkdlfhfii" # TabsPlus
        # "fdpohaocaechififmbbbbbknoalclacl" # GoFullPage - Full Page Screen Capture

        "dnhpnfgdlenaccegplpojghhmaamnnfp" # Augmented Steam
        "anlikcnbgdeidpacdbdljnabclhahhmd" # Enhanced GitHub
        "ficfmibkjjnpogdcfhfokmihanoldbfe" # File Icons for GitHub and GitLab
        "obnjfbgikjcdfnbnmdamffacjfpankih" # Excalisave

        "idgpnmonknjnojddfkpgkljpfnnfcklj" # ModHeader - Modify HTTP headers
        "nhdogjmejiglipccpnnnanhbledajbpd" # Vue.js devtools
        "fmkadmapgofadopljbjfkapdkoienihi" # React Developer Tools
        "pfefekfhnmbfcofpjojnpmhdakcadeil" # AstroSpect
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

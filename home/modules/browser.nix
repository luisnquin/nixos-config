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
        "idgpnmonknjnojddfkpgkljpfnnfcklj" # ModHeader - Modify HTTP headers
        "nhdogjmejiglipccpnnnanhbledajbpd" # Vue.js devtools
        "fdpohaocaechififmbbbbbknoalclacl" # GoFullPage - Full Page Screen Capture
        "gppongmhjkpfnbhagpmjfkannfbllamg" # Wappalyzer - Technology profiler
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

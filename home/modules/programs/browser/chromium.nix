{pkgs, ...}: {
  # check chromium-nixos.nix
  programs.chromium = {
    enable = true;
    package = pkgs.chromium;
    commandLineArgs = [
      "--new-window"
      "https://protect-ue.ismartlife.me/playback"
      "--enable-logging=stderr"
      "--extension-mime-request-handling=always-prompt-for-install"
      "--no-default-browser-check"
      "--enable-zero-copy"
      "--disable-sync-preferences"
      "--ignore-gpu-blocklist"
      "--enable-features=AcceleratedVideoDecodeLinuxGL"
      "--enable-parallel-downloading"
    ];
    extensions = let
      ids = [
        "ficfmibkjjnpogdcfhfokmihanoldbfe" # File Icons for GitHub and GitLab
        "ddkjiahejlhfcafbddmgiahcphecmpfh" # uBlock Origin Lite
        "dffbjiomnajbmlhjelpipfldgkijdemn" # URL Cleaner
        "ldpochfccmkkmhdbclfhpagapcfdljkj" # Decentraleyes
        "mefhakmgclhhfbdadeojlkbllmecialg" # Tabby Cat
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

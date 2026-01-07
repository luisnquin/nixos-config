{pkgs, ...}: {
  # check chromium-nixos.nix
  programs.chromium = {
    enable = true;
    package = pkgs.chromium;
    commandLineArgs = [
      "--restore-last-session=false"
      "--enable-logging=stderr"
      "--extension-mime-request-handling=always-prompt-for-install"
      "--no-default-browser-check"
      "--enable-zero-copy"
      "--disable-sync-preferences"
      "--ignore-gpu-blocklist"
      "--enable-features=AcceleratedVideoDecodeLinuxGL"
      "--enable-parallel-downloading"
      "--password-store=basic"
      " --disable-session-crashed-bubble"
      "--use-mock-keychain"
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

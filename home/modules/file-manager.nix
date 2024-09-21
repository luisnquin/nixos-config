{
  pkgs-beta,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgs-beta.ranger
    nautilus
  ];

  programs.xplr = {
    enable = true;
    package = pkgs.xplr;
    plugins = {
      wl-clipboard = pkgs.fetchFromGitHub {
        owner = "sayanarijit";
        repo = "wl-clipboard.xplr";
        rev = "a3ffc87460c5c7f560bffea689487ae14b36d9c3";
        hash = "sha256-I4rh5Zks9hiXozBiPDuRdHwW5I7ppzEpQNtirY0Lcks=";
      };
    };

    extraConfig = ''
      require("wl-clipboard").setup {
        copy_command = "wl-copy -t text/uri-list",
        paste_command = "wl-paste",
        keep_selection = true,
      }
    '';
  };
}

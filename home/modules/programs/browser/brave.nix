{pkgs, ...}: {
  programs.chromium = {
    enable = true;
    package = pkgs.brave;
    extensions = let
      ids = [
        "ficfmibkjjnpogdcfhfokmihanoldbfe" # File Icons for GitHub and GitLab
      ];
    in
      builtins.map (id: {inherit id;}) ids;
  };
}

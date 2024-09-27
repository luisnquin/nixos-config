{pkgs, ...}: {
  home = {
    shellAliases = rec {
      k9s = "k9s --crumbsless --logoless --refresh=2";
      kw = "k9s";
      k = "kubectl";
      mrk = k; # sometimes I want to call it like this
    };

    packages = with pkgs; [
      kubernetes
      minikube
      kubectl
    ];
  };

  programs.k9s = let
    inherit (pkgs.libx.formats) fromYAML;
  in {
    enable = true;
    skins.skin = fromYAML ../../../dots/k9s/skin.yml;
    views = fromYAML ../../../dots/k9s/views.yml;
  };
}

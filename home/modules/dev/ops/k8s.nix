{
  pkgs,
  libx,
  ...
}: {
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

  programs.k9s = {
    enable = true;
    skins.skin = libx.formats.fromYAML ../../../dots/k9s/skin.yml;
    views = libx.formats.fromYAML ../../../dots/k9s/views.yml;
  };
}

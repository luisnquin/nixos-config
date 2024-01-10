{libx, ...}: {
  programs.k9s = {
    enable = true;
    skin = libx.formats.fromYAML ../../../dots/k9s/skin.yml;
    views = libx.formats.fromYAML ../../../dots/k9s/views.yml;
  };
}

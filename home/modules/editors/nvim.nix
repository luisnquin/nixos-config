{
  neovim-flake,
  pkgs,
  ...
}: let
  configModule.config = {
    build = {
      viAlias = true;
      vimAlias = true;
    };

    vim = {
      autoIndent = true;
      autocomplete.enable = true;

      theme = {
        enable = true;
        name = "tokyonight";
      };

      languages = {
        nix.enable = true;
        rust.enable = true;
        sql.enable = true;
        markdown.enable = true;
      };
    };
  };
in {
  home.packages = [
    (
      neovim-flake.lib.neovimConfiguration {
        modules = [configModule];
        inherit pkgs;
      }
    )
  ];
}

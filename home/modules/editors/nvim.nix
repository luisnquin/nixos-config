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
        enableLSP = true;

        bash.enable = true;
        clang.enable = true;
        zig.enable = true;
        nix.enable = true;
        rust.enable = true;
        go.enable = true;
        ts.enable = true;
        sql = {
          enable = true;
          lsp.enable = false; # (sqls is deprecated, use sqlls instead.)
        };
        terraform.enable = true;
        markdown.enable = true;
        python.enable = true;
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

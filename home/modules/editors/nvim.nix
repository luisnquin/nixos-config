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

    # https://github.com/jordanisaacs/neovim-flake/tree/main/modules
    vim = {
      autoIndent = true;
      disableArrows = false;
      lineNumberMode = "number";
      cmdHeight = 1;
      tabWidth = 4;

      startPlugins = with pkgs.vimPlugins; [
        transparent-nvim
        presence-nvim
        vim-astro
      ];

      luaConfigRC.transparent-nvim-setup = neovim-flake.lib.nvim.dag.entryAnywhere ''
        vim.g.transparent_groups = vim.list_extend(
          vim.g.transparent_groups or {},
          vim.tbl_map(function(v)
            return v.hl_group
          end, vim.tbl_values(require('bufferline.config').highlights))
        )
      '';

      autocomplete.enable = true;

      theme = {
        enable = true;
        name = "tokyonight";
        style = "night";
      };

      keys = {
        enable = true;
        whichKey.enable = true;
      };

      telescope = {
        enable = true;
        fileBrowser = {
          enable = true;
          hijackNetRW = true;
        };

        liveGrepArgs = {
          enable = true;
          autoQuoting = true;
        };
      };

      languages = {
        enableLSP = true;

        bash.enable = true;
        clang.enable = true;
        zig.enable = true;
        nix.enable = true;
        rust.enable = true;
        go = {
          lsp.enable = true;
          treesitter.enable = true;
        };
        ts.enable = true;
        sql = {
          enable = true;
          lsp.enable = false; # (sqls is deprecated, use sqlls instead.)
        };
        terraform.enable = true;
        markdown.enable = true;
        python.enable = true;
      };

      filetree.nvimTreeLua = {
        enable = true;
        treeSide = "left";
        treeWidth = 12;
        hideFiles = [".git" "node_modules" ".cache"];
        hideIgnoredGitFiles = false;
      };

      statusline.lualine = {
        enable = true;
        icons = true;
        theme = "nightfly";
      };

      git = {
        enable = true;
        gitsigns.enable = true;
      };

      visuals.nvimWebDevicons.enable = true;
      tabline.nvimBufferline.enable = true;
      treesitter.context.enable = true;
      snippets.vsnip.enable = true;
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

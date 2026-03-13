{pkgs, ...}: {
  home = let
    mcpConfig = {
      mcpServers = {
        encore = {
          command = "encore";
          args = ["mcp" "run" "--app=gate-k9-mzni"];
          disabledTools = [
            "query_database"
            "get_secrets"
          ];
        };

        supabase = {
          serverUrl = "https://mcp.supabase.com/mcp?project_ref=mjkvxcziwkwohxpuejak";
          disabledTools = [
            "reset_branch"
            "rebase_branch"
            "delete_branch"
            "merge_branch"
            "list_migrations"
            "apply_migration"
            "list_branches"
            "create_branch"
            "deploy_edge_function"
            "get_edge_function"
            "list_edge_functions"
            "execute_sql"
          ];
        };
      };
    };
  in {
    shellAliases."code" = "antigravity";
    file.".gemini/antigravity/mcp_config.json".text = builtins.toJSON mcpConfig;
  };

  programs.vscode = {
    enable = true;
    package = pkgs.antigravity;

    mutableExtensionsDir = true;

    profiles.default = {
      enableExtensionUpdateCheck = true;
      enableUpdateCheck = true;

      enableMcpIntegration = true;

      keybindings = [
        {
          key = "ctrl+c";
          command = "editor.action.clipboardCopyAction";
          when = "textInputFocus";
        }
        {
          key = "ctrl+y";
          command = "redo";
          when = "textInputFocus";
        }
      ];

      extensions = with pkgs.vscode-extensions;
        [
          # vira.vsc-vira-theme
          aaron-bond.better-comments
          arrterian.nix-env-selector
          astro-build.astro-vscode
          bbenoist.nix
          biomejs.biome
          bradlc.vscode-tailwindcss
          brettm12345.nixfmt-vscode
          christian-kohler.path-intellisense
          docker.docker
          eamodio.gitlens
          golang.go
          hashicorp.terraform
          irongeek.vscode-env
          jnoortheen.nix-ide
          kamadorueda.alejandra
          mads-hartmann.bash-ide-vscode
          mechatroner.rainbow-csv
          ms-python.python
          ms-vscode.cpptools
          ms-vscode.cpptools-extension-pack
          prisma.prisma
          redhat.vscode-yaml
          ritwickdey.liveserver
          rust-lang.rust-analyzer
          streetsidesoftware.code-spell-checker
          usernamehw.errorlens
          wmaurer.change-case
          yoavbls.pretty-ts-errors
          zhuangtongfa.material-theme
          ziglang.vscode-zig
        ]
        ++ pkgs.vscode-utils.extensionsFromVscodeMarketplace [
          {
            name = "autopep8";
            publisher = "ms-python";
            version = "2025.2.0";
            sha256 = "sha256-N1ryz3MieHinTcm5d1RVbGApMQAUhrDUpxDUdfEDmvU=";
          }
          {
            name = "arrange-selection";
            publisher = "wupb";
            version = "1.3.1";
            sha256 = "sha256-wo6Lq9i+wkuhYte4nLugCTTOITm4Nfcy5by2NK2/g/M=";
          }
          {
            name = "bash-beautify";
            publisher = "shakram02";
            version = "0.1.1";
            sha256 = "sha256-pg1nGEk+cn7VlmJeDifXkXeZJLRrEFOyW0bK9W6VGfc=";
          }
          {
            name = "concise-sql-formatter";
            publisher = "kaanaytekin";
            version = "0.0.3";
            sha256 = "sha256-I5dkJlFH5JFzQ0Q5rOklVJ1qAzkbSVEy0gxsHH2Adf0=";
          }
          {
            name = "nix-embedded-highlighter";
            publisher = "atomicspirit";
            version = "0.0.1";
            sha256 = "sha256-KZfUaPjReHQH0XCCiejAs+0Go8WEeGiOuxjkTfSnku0=";
          }
        ];
    };
  };
}

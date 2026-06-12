{
  inputs,
  mkAgentKit,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};

  mkVscodeModule = import "${inputs.home-manager}/modules/programs/vscode/mkVscodeModule.nix";
in {
  disabledModules = [
    "programs/antigravity.nix"
  ];

  imports = [
    (mkVscodeModule {
      modulePath = ["programs" "antigravity"];
      name = "Antigravity IDE";
      packageName = "antigravity";
      nameShort = "Antigravity IDE";
      dataFolderName = ".antigravity-ide";
      skipVersionCheck = true;
    })
  ];

  home = {
    shellAliases."code" = "antigravity-ide";

    activation.copyAntigravitySettings = lib.hm.dag.entryAfter ["writeBoundary"] ''
      install -Dm644 ${./settings.json} "$HOME/.config/Antigravity IDE/User/settings.json"
      chmod u+w "$HOME/.config/Antigravity IDE/User/settings.json"
    '';

    file.".gemini/antigravity/mcp_config.json".text = builtins.toJSON (kit.mkMcpServers {
      normalizeServerUrl = true;
    });
  };

  programs.antigravity = {
    enable = true;
    # package = pkgs.llm-agents.antigravity;
    package = pkgs.antigravity;
    mutableExtensionsDir = false;

    profiles.default = {
      enableExtensionUpdateCheck = true;
      enableUpdateCheck = true;

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
        {
          key = "ctrl+space";
          command = "workbench.action.terminal.toggleTerminal";
        }
        {
          key = "insert";
          command = "-toggleOverwriteMode";
        }
      ];

      extensions = with pkgs.vscode-extensions;
        [
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
          esbenp.prettier-vscode
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
          sumneko.lua
          usernamehw.errorlens
          wmaurer.change-case
          yoavbls.pretty-ts-errors
          zhuangtongfa.material-theme
          ziglang.vscode-zig
        ]
        ++ pkgs.vscode-utils.extensionsFromVscodeMarketplace [
          {
            name = "vsc-vira-theme";
            publisher = "vira";
            version = "2026.5.5";
            sha256 = "sha256-CWowBwqHc4tMB1ownZTL0t9SzX6lIcMhkyicPmoPqxw=";
          }
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
          {
            name = "native-preview";
            publisher = "TypeScriptTeam";
            version = "0.20260612.1";
            sha256 = "sha256-CNvY7i7FDz4s2I/DVm4kzDRVUNdW+CuPOZSlvGczgRs=";
          }
        ];
    };
  };
}

{
  mkAgentKit,
  pkgs,
  lib,
  ...
}: let
  kit = mkAgentKit {};
in {
  home.file.".gemini/antigravity/mcp_config.json".text = builtins.toJSON (kit.mkMcpServers {
    normalizeServerUrl = true;
  });

  home.activation.copyAntigravitySettings = lib.hm.dag.entryAfter ["writeBoundary"] ''
    mkdir -p ~/.config/Antigravity/User
    rm -f ~/.config/Antigravity/User/settings.json
    cp ${./settings.json} ~/.config/Antigravity/User/settings.json
    chmod +w ~/.config/Antigravity/User/settings.json
  '';

  home.shellAliases."code" = "antigravity";

  programs.antigravity = {
    enable = true;
    # package = pkgs.llm-agents.antigravity;
    package = pkgs.antigravity;
    mutableExtensionsDir = true;

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
            version = "0.20260329.1";
            sha256 = "sha256-sKePls4n8bgYtSRXLvSTK8bamN/4CVqD2SBcMpytJZg=";
          }
          {
            # it's not working
            name = "maestro-workbench";
            publisher = "Mastersam";
            version = "0.9.4";
            sha256 = "sha256-yiVsyZKCXrjNsM1c3oCb+BLO14IW06DOesem0AfZqsw=";
          }
        ];
    };
  };
}

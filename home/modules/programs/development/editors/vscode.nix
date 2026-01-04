{pkgs, ...}: {
  programs.vscode = {
    enable = true;

    mutableExtensionsDir = true;

    profiles.default = {
      enableExtensionUpdateCheck = true;
      enableUpdateCheck = true;

      extensions = with pkgs.vscode-extensions;
        [
          # kilocode.kilo-code
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
        ];
    };
  };
}

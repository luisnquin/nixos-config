{pkgs, ...}: {
  programs.vscode = {
    enable = true;
    mutableExtensionsDir = true;

    profiles.default = {
      enableExtensionUpdateCheck = true;
      enableUpdateCheck = true;

      extensions = with pkgs.vscode-extensions; [
        # Go
        golang.go

        # Rust
        rust-lang.rust-analyzer

        # Nix
        arrterian.nix-env-selector
        brettm12345.nixfmt-vscode
        kamadorueda.alejandra
        jnoortheen.nix-ide
        bbenoist.nix

        kilocode.kilo-code

        # Python
        ms-python.python

        # Git
        eamodio.gitlens

        # Formats
        redhat.vscode-yaml
        # dlasagno.rasi

        # Theme
        zhuangtongfa.material-theme

        # Misc
        christian-kohler.path-intellisense
        mechatroner.rainbow-csv
        usernamehw.errorlens
        ritwickdey.liveserver
      ];
    };
  };
}

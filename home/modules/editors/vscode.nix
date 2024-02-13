{
  pkgs-latest,
  pkgs,
  ...
}: {
  programs.vscode = {
    enable = true;
    enableExtensionUpdateCheck = true;
    mutableExtensionsDir = true;
    enableUpdateCheck = true;
    package = pkgs-latest.vscode;

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

      # OCaml
      ocamllabs.ocaml-platform

      # Python
      ms-python.python
      ms-python.vscode-pylance
      # ms-python.pylint
      # zeshuaro.vscode-python-poetry

      # JS ecosystem
      # rvest.vs-code-prettier-eslint
      # YoavBls.pretty-ts-errors

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
}

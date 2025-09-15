{
  pkgs,
  system,
  ...
}: {
  programs.vscode = {
    enable = true;
    package = let
      version = "1.104.0";
      throwSystem = throw "Unsupported system: ${system}";
      plat =
        {
          x86_64-linux = "linux-x64";
          x86_64-darwin = "darwin";
          aarch64-linux = "linux-arm64";
          aarch64-darwin = "darwin-arm64";
          armv7l-linux = "linux-armhf";
        }
    .${
          system
        } or throwSystem;
    in
      pkgs.vscode.overrideAttrs (_oldAttrs: {
        src = builtins.fetchTarball {
          url = "https://update.code.visualstudio.com/${version}/${plat}/stable";
          sha256 = "1319wjggg77fs6kgw8jfdjxifgg7sdhw51r5spspsldklk1a7qih";
        };
        inherit version;
      });
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

        # OCaml
        ocamllabs.ocaml-platform

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

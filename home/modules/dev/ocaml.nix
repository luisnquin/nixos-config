{
  config,
  user,
  pkgs,
  lib,
  ...
}: {
  home.packages = with pkgs; [
    ocamlPackages.utop
    dune-release
    ocamlformat
    ocaml
  ];

  programs = {
    opam = {
      enable = true;
      package = pkgs.opam;
      enableZshIntegration = true;
    };

    zsh.initExtra = ''
      [[ ! -r /home/"$USER"/.opam/opam-init/init.zsh ]] || source /home/"$USER"/.opam/opam-init/init.zsh >/dev/null 2>/dev/null
    '';
  };

  systemd.user.services = lib.mkIf config.programs.opam.enable {
    opam-update = {
      Unit.Description = "Automatically update and keep installed opam packages";

      Service = let
        opam = pkgs.opam.overrideAttrs (
          p: {
            postInstall = let
              wrapperLine = ''
                wrapProgram ${placeholder "out"}/bin/opam \
                  --prefix PATH : ${lib.makeBinPath (with pkgs; [gnupatch gnutar gnumake gzip gcc])}
              '';
            in
              p.postInstall
              + wrapperLine;
          }
        );

        packagesToInstall = lib.strings.concatMapStrings (p: p + " ") [
          "ocaml-lsp-server"
          "dune"
          "base"
          "core"
        ];

        opam-init = pkgs.writeShellScriptBin "opam-init" ''
          sudo -Hu ${user.alias} bash -c '${opam}/bin/opam init --no-setup --reinit'
          sudo -Hu ${user.alias} bash -c '${opam}/bin/opam install ${packagesToInstall}'
        '';
      in {
        Type = "oneshot";
        ExecStart = "${opam-init}/bin/opam-init";
        Restart = "on-failure";
      };

      Install = {
        WantedBy = ["default.target"];
      };
    };
  };
}

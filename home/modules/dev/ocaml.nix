{
  pkgs,
  lib,
  ...
}: {
  home = {
    packages = with pkgs; [
      ocamlPackages.utop
      dune-release
      ocamlformat
      ocaml
    ];

    activation.opamInit = let
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
      ];
    in
      lib.hm.dag.entryAfter ["writeBoundary"] ''
        ${opam}/bin/opam init --no-setup --reinit
        ${opam}/bin/opam install ${packagesToInstall}
      '';
  };

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
}

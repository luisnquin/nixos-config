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
    in
      lib.hm.dag.entryAfter ["writeBoundary"] ''
        ${opam}/bin/opam init --no-setup --reinit
        ${opam}/bin/opam install ocaml-lsp-server
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

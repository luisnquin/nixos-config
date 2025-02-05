{
  user,
  pkgs,
  ...
}: let
  opamInitScriptPath = "/home/${user.alias}/.opam/opam-init/init.zsh";
in {
  home.packages = with pkgs; [
    ocamlPackages.utop
    dune-release
    ocamlformat
    dune_3
    ocaml
  ];

  programs = {
    opam = {
      enable = true;
      package = pkgs.opam;
      enableZshIntegration = true;
    };

    zsh.initExtra = ''
      [[ ! -r ${opamInitScriptPath} ]] || source ${opamInitScriptPath} >/dev/null 2>/dev/null
    '';
  };

  # systemd.user.services = lib.mkIf config.programs.opam.enable {
  #   opam-update = {
  #     Unit.Description = "Automatically update and keep installed opam packages";

  #     Service = let
  #       opam = pkgs.opam.overrideAttrs (
  #         p: {
  #           postInstall = let
  #             wrapperLine = ''
  #               wrapProgram ${placeholder "out"}/bin/opam \
  #                 --prefix PATH : ${lib.makeBinPath (with pkgs; [gnupatch gnutar gnumake gzip gcc])}
  #             '';
  #           in
  #             p.postInstall
  #             + wrapperLine;
  #         }
  #       );

  #       packagesToInstall = lib.strings.concatMapStrings (p: p + " ") [
  #         "ocaml-lsp-server"
  #         "base"
  #         "core"
  #       ];

  #       opam-init = pkgs.writeShellScriptBin "opam-init" ''
  #         if ! test -f ${opamInitScriptPath}; then
  #           return
  #         fi

  #         eval $(${opam}/bin/opam env)
  #         source ${opamInitScriptPath}

  #         sudo -Hu ${user.alias} ${pkgs.bash}/bin/bash -c '${opam}/bin/opam install -y ${packagesToInstall}'
  #       '';
  #     in {
  #       Type = "oneshot";
  #       ExecStart = "${opam-init}/bin/opam-init";
  #       Restart = "never";
  #     };

  #     Install = {
  #       WantedBy = ["default.target"];
  #     };
  #   };
  # };
}

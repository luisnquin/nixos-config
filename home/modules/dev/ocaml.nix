{pkgs, ...}: {
  home.packages = with pkgs; [
    ocamlPackages.utop
    dune-release
    ocamlformat
    ocaml
  ];

  programs.opam = {
    enable = true;
    package = pkgs.opam;
    enableZshIntegration = true;
  };
}

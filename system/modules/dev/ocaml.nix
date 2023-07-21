{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    dune-release
    ocamlformat
    opam
  ];
}

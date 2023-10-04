{pkgs, ...}: {
  home.packages = with pkgs; [
    (python310.withPackages
      (p:
        with p; [
          pipx
          pip
        ]))
    virtualenv
    poetry
    pyright
  ];
}

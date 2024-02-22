{pkgs, ...}: {
  home.packages = with pkgs; [
    (python310.withPackages
      (p:
        with p; [
          libtmux
          pipx # install and run apps in isolated environments
          pip
        ]))
    virtualenv # to create raw environments
    poetry # package manager
    pyright # linter
  ];
}

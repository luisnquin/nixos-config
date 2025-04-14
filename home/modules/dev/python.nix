{pkgs, ...}: {
  home.packages = with pkgs; [
    (python313.withPackages
      (p:
        with p; [
          pipx # install and run apps in isolated environments
          pip
        ]))
    virtualenv # to create raw environments
    poetry # package manager
    pyright # linter
  ];
}

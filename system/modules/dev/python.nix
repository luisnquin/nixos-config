{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    (python310.withPackages
      (p:
        with p; [
          pipx
          pip
        ]))
    virtualenv
    pyright
  ];
}

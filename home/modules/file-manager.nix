{pkgs, ...}: {
  home.packages = [
    pkgs.nautilus
  ];

  programs.ranger = {
    enable = true;

    rifle = [
      {
        condition = "ext wav";
        command = "${pkgs.pulseaudio}/bin/paplay \"$@\"";
      }
    ];
  };
}

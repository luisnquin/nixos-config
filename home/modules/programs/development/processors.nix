{pkgs, ...}: {
  home.packages = with pkgs; [
    htmlq
    yq-go
  ];

  programs.jq = {
    enable = true;
    colors = {
      arrays = "1;37";
      false = "0;37";
      null = "1;30";
      numbers = "0;37";
      objects = "1;37";
      strings = "0;32";
      true = "0;37";
      objectKeys = "1;34";
    };
  };
}

{pkgs, ...}: {
  programs = {
    cliphizt.enable = true;
    cliplenz = {
      enable = true;
      fonts = with pkgs; [
        cascadia-code
        dejavu_fonts
      ];
    };
  };
}

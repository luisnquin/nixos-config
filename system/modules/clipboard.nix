{
  isWayland,
  pkgs,
  ...
}:
if isWayland
then {
  environment.systemPackages = with pkgs; [
    wl-clipboard
    cliphist
  ];
}
else {
  environment = {
    systemPackages = [pkgs.xclip];

    shellAliases = {
      xclip = "xclip -selection c";
    };
  };
}

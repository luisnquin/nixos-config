{
  isWayland,
  pkgs,
  ...
}:
if isWayland
then {
  environment.systemPackages = with pkgs; [
    wl-clipboard
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

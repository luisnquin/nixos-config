{
  isWayland,
  pkgs,
  ...
}:
if isWayland
then {
  environment.systemPackages = with pkgs; [
    wl-clipboard
    pkgs.za.cliphist
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

{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  environment.systemPackages = with pkgs; [
    cargo
    clippy
    rust-analyzer
    rustc
    rustfmt
    rustup
    vscode-extensions.matklad.rust-analyzer
  ];
}
# fileSystems."${pkgs.rustc}".permissions = "u:${owner.username}:rwx";
# systemd.services.rustc-permissions = {
#   enable = true;
#   description = "Set read, write, and execute permissions for rustsrc files in /nix/store";
#   serviceConfig = {
#     Type = "oneshot";
#     ExecStart = ''${pkgs.bash}/bin/bash -c "chmod -R u+rwx ${pkgs.rustc}"'';
#     ExecStop = ''${pkgs.bash}/bin/bash -c "exit 0"'';
#   };
#   wantedBy = ["multi-user.target"];
# };
#postStart = ''
#  chmod -R u+rwx ${pkgs.rustc}
#'';
# services.systemd.services = ["rustc-permissions"];
# fileSystems."${pkgs.rustc}".permissions = "u:${config.system.build.username}:rwx";


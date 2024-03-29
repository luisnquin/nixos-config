{
  pkgs ? import <nixpkgs> {},
  notificationIcon ? ./assets/nix-logo.png,
  hyprlandSupport ? false,
}:
assert builtins.isPath notificationIcon;
assert builtins.isBool hyprlandSupport; let
  runtimePackages = with pkgs; [
    libnotify
    alejandra
    eza
  ];

  inherit (pkgs) lib;
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = builtins.path {
      inherit name;
      path = ./.;
    };

    nativeBuildInputs = [pkgs.makeWrapper];

    propagatedBuildInputs = runtimePackages;

    postPatch = ''
      substituteInPlace ./main.bash \
        --replace '/path/to/nix-logo.png' '${notificationIcon}' \
        --replace 'USES_HYPRLAND=false' 'USES_HYPRLAND=${lib.boolToString hyprlandSupport}' \
        --replace '#! /usr/bin/env nix-shell\n#! nix-shell -i bash -p bash alejandra home-manager git' '#! /usr/bin/env bash'
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin

      cp ./main.bash $out/bin/nyx
      chmod +x $out/bin/nyx

      runHook postInstall
    '';

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/nyx \
        --prefix PATH : ${lib.makeBinPath runtimePackages}
    '';
  }

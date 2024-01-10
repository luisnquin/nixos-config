{
  pkgs ? import <nixpkgs> {},
  notificationIcon ? ./assets/nix-logo.png,
  dotfilesDir ? "$HOME/.dotfiles",
  hyprlandSupport ? false,
}:
assert builtins.isPath notificationIcon;
assert builtins.isBool hyprlandSupport;
assert builtins.isString dotfilesDir; let
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

    postPatch = let
      usesHyprlandExpr = "USES_HYPRLAND=\"${lib.boolToString hyprlandSupport}\"";
    in ''
      substituteInPlace ./main.bash \
        --replace '/path/to/nix-logo.png' '${notificationIcon}'

      substituteInPlace ./main.bash \
        --replace 'USES_HYPRLAND="false"' '${usesHyprlandExpr}}'
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
        --prefix PATH : ${lib.makeBinPath runtimePackages} \
        --prefix DOTFILES_PATH : ${dotfilesDir}
    '';
  }

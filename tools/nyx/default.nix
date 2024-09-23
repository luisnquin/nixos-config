{
  pkgs ? import <nixpkgs> {},
  notificationIcon ? ./assets/nix-logo.png,
}:
assert builtins.isPath notificationIcon; let
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
        --replace-fail '/path/to/nix-logo.png' '${notificationIcon}'
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

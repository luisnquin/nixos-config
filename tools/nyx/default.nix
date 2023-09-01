{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = builtins.path {
      inherit name;
      path = ./.;
    };

    nativeBuildInputs = with pkgs; [
      makeWrapper
    ];

    propagatedBuildInputs = with pkgs; [
      alejandra
      exa
    ];

    postPatch = ''
      substituteInPlace ./main.sh \
        --replace '/path/to/nix-logo.png' '${placeholder "out"}/assets/nix-logo.png'
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/

      mkdir -p $out/bin/
      cp ./main.sh $out/bin/nyx
      chmod +x $out/bin/nyx

      mkdir -p $/out/assets/
      cp -r $src/assets/ $out/assets/

      runHook postInstall
    '';

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/nyx \
        --prefix PATH : ${pkgs.lib.makeBinPath (with pkgs; [exa alejandra])}
    '';

    # substituteInPlace ${placeholder "out"}/bin/nyx \
    #   --replace '/path/to/nix-logo.png' '${placeholder "out"}/assets/nix-logo.png'

    meta = with pkgs.lib; {
      description = "A script to manage my NixOS computer";
      homepage = "https://github.com/${owner}/nixos-config";
      license = licenses.unlicense;
      maintainers = with maintainers; [luisnquin];
    };
  }

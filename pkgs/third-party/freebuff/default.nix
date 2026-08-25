{
  lib,
  stdenv,
  fetchFromGitHub,
  bun2nix,
  ripgrep,
  makeWrapper,
}:
let
  version = "0.0.154";
  src = fetchFromGitHub {
    owner = "CodebuffAI";
    repo = "freebuff";
    rev = "35be6609aceadbb8012fa99a2797b0f7497b6e04";
    hash = "sha256-9z4aLORdQNJpVgxHNymlcHYx+H3z51J7u6yRrkCJNyk=";
  };

  bunDeps = bun2nix.fetchBunDeps {
    bunNix = ./bun.nix;
  };

  receipt-glibc = ./opentui-receipt-glibc.json;
  receipt-musl = ./opentui-receipt-musl.json;

  publicEnv = {
    NEXT_PUBLIC_CB_ENVIRONMENT = "prod";
    NEXT_PUBLIC_CODEBUFF_APP_URL = "https://www.codebuff.com";
  };
in
stdenv.mkDerivation (
  publicEnv
  // {
    pname = "freebuff";
    inherit version src bunDeps;

    nativeBuildInputs = [
      bun2nix.hook
      makeWrapper
    ];

    patches = [
      ../../../overlays/patches/freebuff/branded-header-and-input.patch
      ../../../overlays/patches/freebuff/cli-only-client-env.patch
      ../../../overlays/patches/freebuff/remove-ascii-banner.patch
    ];

    buildPhase = ''
      runHook preBuild

      export HOME=$TMPDIR
      export FREEBUFF_MODE=true

      cp ${receipt-glibc} node_modules/@opentui/core-linux-x64/.freebuff-native-bundle.json
      cp ${receipt-musl} node_modules/@opentui/core-linux-x64-musl/.freebuff-native-bundle.json

      cd sdk && bun run build && cd ..
      bun freebuff/cli/build.ts ${version}

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin
      install -m755 ./cli/bin/freebuff $out/bin/freebuff
      install -m644 ./cli/bin/tree-sitter.wasm $out/bin/tree-sitter.wasm
      wrapProgram $out/bin/freebuff \
        --argv0 freebuff \
        --prefix PATH : ${lib.makeBinPath [ ripgrep ]}

      runHook postInstall
    '';

    bunInstallFlags = [ "--linker=hoisted" ];

    dontUseBunBuild = true;
    dontUseBunInstall = true;

    dontStrip = true;
    doInstallCheck = false;

    meta = with lib; {
      description = "The world's strongest free coding agent - built from source";
      homepage = "https://freebuff.com";
      license = licenses.asl20;
      platforms = [ "x86_64-linux" ];
      mainProgram = "freebuff";
    };
  }
)

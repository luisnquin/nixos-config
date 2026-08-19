{
  rustPlatform,
  fetchFromGitHub,
}:
# Mic92's OSC 52 fork of rmarganti/herdr-pluck: clipboard survives SSH panes.
rustPlatform.buildRustPackage {
  pname = "herdr-pluck";
  version = "0.1.0-unstable-2026-07-23";

  src = fetchFromGitHub {
    owner = "Mic92";
    repo = "herdr-pluck";
    rev = "6f94c5b2e41e3f51a868847d7a62f140c4ff496c";
    hash = "sha256-7MyNBAHUbimRd68Oj8d9Y2l4knmHMqHNNdUtBJOkwJM=";
  };

  cargoHash = "sha256-h3yU5gPuJSdv4fW8kbfCxdAR0Nnnr5/dYTNaMhNNFIE=";

  postInstall = ''
    cp herdr-plugin.toml $out/
  '';

  meta.mainProgram = "herdr-pluck";
}

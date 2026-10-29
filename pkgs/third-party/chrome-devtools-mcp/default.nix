{
  lib,
  stdenvNoCC,
  fetchurl,
  makeWrapper,
  nodejs,
}:
stdenvNoCC.mkDerivation rec {
  pname = "chrome-devtools-mcp";
  version = "1.12.0";

  # The registry tarball ships its dependencies bundled under build/, so it
  # runs as is with no lockfile or npm install.
  src = fetchurl {
    url = "https://registry.npmjs.org/chrome-devtools-mcp/-/chrome-devtools-mcp-${version}.tgz";
    hash = "sha256-Sq3jFnw+eBx5fWEJ7WA6DKW4yzY6KJ4uqsqpsMXSo2A=";
  };

  nativeBuildInputs = [makeWrapper];

  dontBuild = true;

  installPhase = ''
    runHook preInstall

    mkdir -p $out/lib/chrome-devtools-mcp $out/bin
    cp -r build package.json $out/lib/chrome-devtools-mcp
    for bin in chrome-devtools-mcp chrome-devtools; do
      makeWrapper ${lib.getExe nodejs} $out/bin/$bin \
        --add-flags $out/lib/chrome-devtools-mcp/build/src/bin/$bin.js \
        --set CHROME_DEVTOOLS_MCP_NO_UPDATE_CHECKS 1 \
        --set CHROME_DEVTOOLS_MCP_NO_USAGE_STATISTICS 1
    done

    runHook postInstall
  '';

  meta = {
    description = "MCP server that drives Chrome through the DevTools protocol";
    homepage = "https://github.com/ChromeDevTools/chrome-devtools-mcp";
    license = lib.licenses.asl20;
    mainProgram = "chrome-devtools-mcp";
  };
}

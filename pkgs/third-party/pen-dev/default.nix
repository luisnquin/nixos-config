{
  lib,
  buildNpmPackage,
  fetchurl,
  jq,
  autoPatchelfHook,
  stdenv,
  libxcb,
  ncurses,
}:
buildNpmPackage rec {
  pname = "pen-dev";
  version = "0.3.13";

  src = fetchurl {
    url = "https://registry.npmjs.org/@pen.dev/cli/-/cli-${version}.tgz";
    hash = "sha256-gPrMfTCt9DEk1kEUBGe2p+6sHamKAEWv7FnCwqc2bgY=";
  };

  # The registry tarball ships no lockfile and lists private @ha/* devDependencies,
  # so the lock was generated against a manifest stripped of both.
  postPatch = ''
    cp ${./package-lock.json} package-lock.json
    ${lib.getExe jq} 'del(.devDependencies, .scripts)' package.json > package.json.tmp
    mv package.json.tmp package.json
  '';

  npmDepsHash = "sha256-yi2zbmEIo0S+jcxWPgwOcaVsaIdpIVYgykJMgfyx5OA=";
  npmFlags = ["--ignore-scripts"];
  dontNpmBuild = true;

  nativeBuildInputs = [autoPatchelfHook];
  buildInputs = [stdenv.cc.cc.lib libxcb ncurses];
  autoPatchelfIgnoreMissingDeps = ["libc.musl-x86_64.so.1"];

  meta = {
    description = "CLI for the pen.dev agent that edits .pen design files";
    homepage = "https://pen.dev";
    license = lib.licenses.unfree;
    platforms = ["x86_64-linux"];
    mainProgram = "pen";
  };
}

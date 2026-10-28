# Pi packages built ahead of time so pi loads them as local paths and never
# runs npm. Lockfiles hold runtime deps only; pi provides the peers.
{
  lib,
  buildNpmPackage,
  fetchurl,
  jq,
}: let
  mkPiPackage = {
    pname,
    version,
    hash,
    npmDepsHash,
    homepage,
  }:
    buildNpmPackage {
      inherit pname version npmDepsHash;

      src = fetchurl {
        url = "https://registry.npmjs.org/${pname}/-/${pname}-${version}.tgz";
        inherit hash;
      };

      postPatch = ''
        cp ${./${pname}.lock.json} package-lock.json
        ${lib.getExe jq} 'del(.devDependencies, .scripts)' package.json > package.json.tmp
        mv package.json.tmp package.json
      '';

      npmFlags = ["--ignore-scripts" "--legacy-peer-deps"];
      dontNpmBuild = true;

      passthru.piPath = "lib/node_modules/${pname}";

      meta = {
        inherit homepage;
        license = lib.licenses.mit;
      };
    };
in {
  pi-subagents = mkPiPackage {
    pname = "pi-subagents";
    version = "0.83.0";
    hash = "sha512-MlmLLa8Cb2/RCkkhCgbRDdeigpFL8v0rKqa+HjWpa/3ylZb49Fg+XDsbMeNu+zQ6268dnxGt1sTDhXsAKxBzKg==";
    npmDepsHash = "sha256-4nLUfr9DMGk7z8tJmwN5NLOX2tEF7h0KNlShabGgtKI=";
    homepage = "https://github.com/nicobailon/pi-subagents";
  };

  pi-diff-review = mkPiPackage {
    pname = "pi-diff-review";
    version = "0.1.29";
    hash = "sha512-wAoILWRJ/vEbhYdzAlN6YWrZt7uMCypSIAm3VoJocQeGOPIAIQFMCajFMapuYh+hmI93ksH1DM9hGaDXMOi4bg==";
    npmDepsHash = "sha256-D8o/IeNcMns2Fq+VzyZCbsCCv8VixdXOYGqKtQZeHyw=";
    homepage = "https://github.com/cmpadden/pi-diff-review";
  };
}

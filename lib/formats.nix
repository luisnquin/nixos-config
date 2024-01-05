{pkgs, ...}: {
  fromYAML =
    # Read a YAML file into a Nix datatype using IFD.
    #
    # Similar to:
    #
    # > builtins.fromJSON (builtins.readFile ./somefile)
    #
    # but takes an input file in YAML instead of JSON.
    #
    # readYAML :: Path -> a
    #
    # where `a` is the Nixified version of the input file.
    #
    # Source: https://github.com/cdepillabout/stacklock2nix/blob/65a34bec929e7b0e50fdf4606d933b13b47e2f17/nix/build-support/stacklock2nix/read-yaml.nix
    path: let
      jsonOutputDrv = pkgs.runCommand "from-yaml" {
        nativeBuildInputs = [pkgs.remarshal];
      } "remarshal -if yaml -i \"${path}\" -of json -o \"$out\"";
    in
      builtins.fromJSON (builtins.readFile jsonOutputDrv);
}

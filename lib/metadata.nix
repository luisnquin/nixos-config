{lib, ...}: {
  mkMetadata = flakeToml: usagePredicate: let
    metadataFileError = message: builtins.throw "flake.toml file error: ${message}";
    metadata = builtins.fromTOML (builtins.readFile flakeToml);

    useFileError = message: builtins.throw "use.conf file error: ${message}";
    use = let
      fragments = lib.strings.splitString "@" usagePredicate;
    in
      if builtins.length fragments == 2
      then {
        user = builtins.elemAt fragments 0;
        host = builtins.elemAt fragments 1;
      }
      else useFileError "format must be <user>@<host> (e.g.: max@myhost)";

    notEmptyString = x: x != null && x != "";
  in {
    user = let
      id = use.user;
    in
      if builtins.hasAttr id metadata.users
      then metadata.users."${id}" // {alias = "${id}";}
      else metadataFileError "missing user '${id}' in users collection";

    host = let
      id = use.host;
    in
      if builtins.hasAttr id metadata.hosts
      then metadata.hosts."${id}" // {name = "${id}";}
      else metadataFileError "missing host '${id}' in hosts collection";

    nix =
      if notEmptyString metadata.nix.stateVersion && notEmptyString metadata.nix.channel
      then metadata.nix
      else metadataFileError "missing one or more attributes of 'nix'";
  };
}

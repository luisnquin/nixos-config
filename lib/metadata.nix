{
  mkMetadata = p: let
    metadata = builtins.fromTOML (builtins.readFile p);
    metadataFileError = message: builtins.throw "metadata file error: ${message}";

    notEmptyString = x: x != null && x != "";

    selected =
      if notEmptyString metadata.use
      then metadata.use
      else metadataFileError "missing 'use'";
  in {
    user =
      if builtins.hasAttr "${selected}" metadata.users
      then metadata.users.${selected} // {alias = selected;}
      else metadataFileError "missing '${selected}' owner in users collection";

    host =
      if builtins.hasAttr "${selected}" metadata.hosts
      then metadata.hosts.${selected}
      else metadataFileError "missing '${selected}' owner in hosts collection";

    nix =
      if notEmptyString metadata.nix.stateVersion && notEmptyString metadata.nix.channel
      then metadata.nix
      else metadataFileError "missing one or more attributes of 'nix'";
  };
}

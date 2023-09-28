{lib}: {
  # Retrieves a list of file paths within the specified directory path.
  #
  # Arguments:
  # - folderPath: A path to the directory.
  #
  # Returns:
  # A list of regular file paths within the directory.
  getFilesInDirectory = with builtins;
    folderPath: let
      inputError = message: throw "input error: ${message}";

      fileNames =
        if lib.isPath folderPath
        then
          if readFileType folderPath == "directory"
          then let
            rawEntries = readDir folderPath;
            entries = lib.attrsets.filterAttrs (_n: v: v == "regular") rawEntries;
          in
            attrNames entries
          else inputError "expected a path to folder"
        else inputError "value is not a path";
    in
      map (p: folderPath + ("/" + p)) fileNames;
}

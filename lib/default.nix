{lib}: {
  getFolderPaths = with builtins;
    folderPath: let
      inputError = message: throw "input error: ${message}";

      fileNames =
        if lib.isPath folderPath
        then
          if readFileType folderPath == "directory"
          then attrNames (readDir folderPath)
          else inputError "expected a path to folder"
        else inputError "value is not a path";
    in
      map (p: folderPath + ("/" + p)) fileNames;
}

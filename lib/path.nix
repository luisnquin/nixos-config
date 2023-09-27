{
  getFolderPaths = with builtins;
    folderPath: let
      fileNames = attrNames (readDir folderPath);
    in
      map (p: folderPath + ("/" + p)) fileNames;
}

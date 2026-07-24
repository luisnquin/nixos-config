{
  programs.spiceedit = {
    enable = true;

    formatters = {
      go = ["gofmt" "-w" "$FILE"];
      json = ["biome" "format" "--write" "$FILE"];
    };
  };
}

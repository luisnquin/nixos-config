{
  home.shellAliases = rec {
    k9s = "k9s --crumbsless --logoless --refresh=2";
    kw = "k9s";
    k = "kubectl";
    mrk = k; # sometimes I want to call it like this
  };
}

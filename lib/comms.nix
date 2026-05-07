{lib, ...}: {
  mkNtfy = host: {
    topic,
    message,
    title ? null,
    tags ? null,
    priority ? 3, # 1..5
  }: let
    cleanHost = lib.removeSuffix "/" host;

    tagValue =
      if tags == null
      then null
      else if builtins.isList tags
      then lib.concatStringsSep "," tags
      else tags;

    headers =
      [
        "-H ${lib.escapeShellArg "Priority: ${toString priority}"}"
      ]
      ++ lib.optional (title != null)
      "-H ${lib.escapeShellArg "Title: ${title}"}"
      ++ lib.optional (tagValue != null)
      "-H ${lib.escapeShellArg "Tags: ${tagValue}"}";

    curlHeaders = lib.concatStringsSep " " headers;
    data = lib.escapeShellArg message;
    url = lib.escapeShellArg "${cleanHost}/${topic}";
  in ''
    curl -fsS ${curlHeaders} -d ${data} ${url} >/dev/null 2>&1 &
  '';
}

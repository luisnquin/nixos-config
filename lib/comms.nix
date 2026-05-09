{lib, ...}: {
  mkNtfy = host: {
    topic,
    message,
    title ? null,
    tags ? null,
    priority ? 3, # 1..5
    delay ? null,
    sequenceId ? null,
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
      "-H ${lib.escapeShellArg "Tags: ${tagValue}"}"
      ++ lib.optional (delay != null)
      "-H ${lib.escapeShellArg "In: ${delay}"}";

    curlHeaders = lib.concatStringsSep " " headers;
    data = lib.escapeShellArg message;
    url = lib.escapeShellArg (
      if sequenceId != null
      then "${cleanHost}/${topic}/${sequenceId}"
      else "${cleanHost}/${topic}"
    );
  in ''
    curl -fsS ${curlHeaders} -d ${data} ${url} >/dev/null 2>&1 &
  '';

  cancelNtfy = host: {
    topic,
    sequenceId,
  }: let
    cleanHost = lib.removeSuffix "/" host;
    url = lib.escapeShellArg "${cleanHost}/${topic}/${sequenceId}";
  in ''
    curl -fsS -X DELETE ${url} >/dev/null 2>&1 &
  '';
}

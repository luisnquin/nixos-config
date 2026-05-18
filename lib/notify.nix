{
  lib,
  pkgs,
}: let
  desktopCommand = {
    image,
    title,
    message,
    appName ? title,
  }:
    lib.concatStringsSep " " [
      (lib.getExe pkgs.libnotify)
      "-a"
      (lib.escapeShellArg appName)
      "-i"
      (lib.escapeShellArg image)
      (lib.escapeShellArg title)
      (lib.escapeShellArg message)
    ];

  ntfySend = {
    host,
    topic,
    message,
    title ? null,
    tags ? null,
    priority ? 3,
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

  ntfyCancel = {
    host,
    topic,
    sequenceId,
  }: let
    cleanHost = lib.removeSuffix "/" host;
    url = lib.escapeShellArg "${cleanHost}/${topic}/${sequenceId}";
  in ''
    curl -fsS -X DELETE ${url} >/dev/null 2>&1 &
  '';
in {
  desktop = desktopCommand;

  ntfy = {
    send = ntfySend;
    cancel = ntfyCancel;
  };

  send = {
    desktop,
    ntfy ? null,
  }: let
    renderedDesktop = desktopCommand desktop;
    ntfyCommand =
      if (ntfy == null) || ((ntfy.host or "") == "")
      then ""
      else
        ntfySend ({
            topic = "notifications";
            title = desktop.title;
            message = desktop.message;
          }
          // ntfy);
  in
    if ntfyCommand == ""
    then renderedDesktop
    else "(${renderedDesktop}) && (${ntfyCommand})";
}

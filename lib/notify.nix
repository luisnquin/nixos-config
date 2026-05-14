{
  lib,
  pkgs,
}: let
  comms = import ./comms.nix {inherit lib;};
in rec {
  /*
  Single notify-send invocation (summary + body), same shape as agent hooks.
  */
  desktopNotifyCmd = image: title: message:
    lib.concatStringsSep " " [
      (lib.getExe pkgs.libnotify)
      "-a"
      (lib.escapeShellArg title)
      "-i"
      (lib.escapeShellArg image)
      (lib.escapeShellArg title)
      (lib.escapeShellArg message)
    ];

  /*
  Desktop notification plus optional ntfy curl (background), matching agents/roborev hooks.
  mergedNtfy is merged into mkNtfy args (topic, title, message, delay, tags, ...).
  */
  notifyShell = ntfyHost: image: title: message: mergedNtfy: let
    base = desktopNotifyCmd image title message;
    merged =
      {
        topic = "notifications";
        inherit title message;
      }
      // mergedNtfy;
    ntfyPart =
      if (ntfyHost == null) || (ntfyHost == "")
      then ""
      else comms.mkNtfy ntfyHost merged;
  in
    if ntfyPart == ""
    then base
    else "(${base}) && (${ntfyPart})";
}

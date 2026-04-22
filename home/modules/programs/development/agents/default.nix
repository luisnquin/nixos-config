{
  pkgs,
  lib,
  ...
}: {
  _module.args = {
    agent = rec {
      assets = import ./assets;

      domains = let
        allowedDomains = builtins.readFile ./.well-known/ai-allowed-domains.txt;
      in
        builtins.filter (s: s != "") (
          lib.strings.splitString "\n" allowedDomains
        );

      memories = builtins.readFile ./.well-known/memories.txt;

      mkAudioCmd = files:
        builtins.concatStringsSep " && " (
          map (mp3: "${pkgs.pulseaudio}/bin/paplay --volume=32768 ${mp3}") files
        );

      mkAudioHook = files: {
        matcher = "";
        hooks = [
          {
            type = "command";
            command = mkAudioCmd files;
          }
        ];
      };

      mkNotificationCmd = image: title: message: ''${lib.getExe pkgs.libnotify} -a "${title}" -i "${image}" "${title}" "${message}"'';

      mkNotificationHook = image: title: message: {
        matcher = "";
        hooks = [
          {
            type = "command";
            command = mkNotificationCmd image title message;
          }
        ];
      };
    };
  };

  imports = [
    ./options
    ./cli/codex
    ./cli/gemini
    ./cli/claude
    ./cli/opencode
    ./mcp.nix
    ./roborev.nix
    ./skills.nix
    ./tools.nix
  ];
}

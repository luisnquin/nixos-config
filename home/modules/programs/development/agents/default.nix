{
  pkgs,
  lib,
  ...
}: {
  _module.args = {
    agent = {
      assets = import ./assets;

      domains = let
        allowedDomains = builtins.readFile ./.well-known/ai-allowed-domains.txt;
      in
        builtins.filter (s: s != "") (
          lib.strings.splitString "\n" allowedDomains
        );

      memories = builtins.readFile ./.well-known/memories.txt;

      mkAudioHook = files: {
        matcher = "";
        hooks = [
          {
            type = "command";
            command = builtins.concatStringsSep " && " (
              map (mp3: "${pkgs.pulseaudio}/bin/paplay ${mp3}") files
            );
          }
        ];
      };

      mkNotificationHook = image: title: message: {
        matcher = "";
        hooks = [
          {
            type = "command";
            command = ''${lib.getExe pkgs.libnotify} -a "${title}" -i "${image}" "${title}" "${message}"'';
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
    ./skills.nix
    ./tools.nix
  ];
}

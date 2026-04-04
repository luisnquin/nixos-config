{lib, ...}: {
  _module.args = {
    agent = {
      assets = import ./assets;

      domains = let
        allowedDomains = builtins.readFile ./.well-known/ai-allowed-domains.txt;
      in
        builtins.filter (s: s != "") (
          lib.strings.splitString "\n" allowedDomains
        );
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

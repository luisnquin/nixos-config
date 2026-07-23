{
  inputs,
  system,
  ...
}:
with inputs; [
  (_final: _prev: {
    home-manager = home-manager.packages.${system}.home-manager.overrideAttrs (old: {
      buildCommand =
        old.buildCommand
        + ''
          substituteInPlace $out/bin/home-manager \
            --replace-fail "    presentNews" "    :"
        '';
    });
  })
  (_final: _prev: {
    llm-agents = llm-agents.packages.${system};
  })
  hyprdysmorphic.overlays.default
  nixpkgs-extra.overlays.default
  agentic-flake.overlays.default
  hermes-agent.overlays.default
  inputs.herdr.overlays.default
  clipz.overlays.default
]

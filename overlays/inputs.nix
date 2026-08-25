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
  bun2nix.overlays.default
  (_final: prev: {
    pythonPackagesExtensions =
      prev.pythonPackagesExtensions
      ++ [
        (_pythonFinal: pythonPrev: {
          nanoemoji = pythonPrev.nanoemoji.overridePythonAttrs (old: {
            src = old.src.overrideAttrs (_: {
              outputHash = "sha256-FysyKC01XBnRiur5RR9fcsTxQqE8x0JJHSoe3q6JtKc=";
            });
          });
        })
      ];
  })
  hyprdysmorphic.overlays.default
  nixpkgs-extra.overlays.default
  agentic-flake.overlays.default
  hermes-agent.overlays.default
  inputs.herdr.overlays.default
]

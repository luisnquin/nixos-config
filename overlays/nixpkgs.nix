{
  inputs,
  system,
}: [
  (
    _self: super: {
      panicparse = super.panicparse.overrideAttrs (_old: {
        postInstall = ''
          mv $out/bin/panicparse $out/bin/pp
        '';
      });
    }
  )
  (
    _self: super: {
      vscode = super.vscode.overrideAttrs (_old: let
        version = "1.106.0";
        throwSystem = throw "Unsupported system: ${system}";

        platEquivs = {
          x86_64-linux = "linux-x64";
          x86_64-darwin = "darwin";
          aarch64-linux = "linux-arm64";
          aarch64-darwin = "darwin-arm64";
          armv7l-linux = "linux-armhf";
        };

        plat = platEquivs.${system} or throwSystem;
      in {
        src = builtins.fetchTarball {
          url = "https://update.code.visualstudio.com/${version}/${plat}/stable";
          sha256 = "1s311sd1cbpg099wdac3di8f75y632xxw5mhjbyqhyb1mwv5cf3l";
        };
        inherit version;
      });
    }
  )
  (_self: _super: {
    inherit (inputs.hyprland.packages.${system}) xdg-desktop-portal-hyprland;

    # fufexan is a noob at nix, look this shit: https://github.com/hyprwm/Hyprland/blob/c92fb5e85f4a5fd3a0f5ffb5892f6a61cfe1be2b/nix/default.nix#L82

    hyprland = inputs.hyprland.packages.${system}.hyprland.overrideAttrs (_oldAttrs: {
      # disko does not work with the src they've set
      src = _self.fetchgit {
        url = "https://github.com/hyprwm/Hyprland";
        rev = "c92fb5e85f4a5fd3a0f5ffb5892f6a61cfe1be2b";
        sha256 = "sha256-sBndsTEWfHREb1bKdEy0RI0qShcVMgOVXguEdLMR7KA=";
      };
    });
  })
]

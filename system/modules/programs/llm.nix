{pkgs, ...}: {
  services = {
    ollama = {
      enable = true;
      package = pkgs.ollama.overrideAttrs (_old: rec {
        # Use the latest version from nixpkgs
        version = "0.12.3";
        src = pkgs.fetchFromGitHub {
          owner = "ollama";
          repo = "ollama";
          tag = version;
          hash = "sha256-ooDGwTklGJ/wzDlAY3uJiqpZUxT1cCsqVNJKU8BAPbQ=";
        };

        vendorHash = "sha256-SlaDsu001TUW+t9WRp7LqxUSQSGDF1Lqu9M1bgILoX4=";
      });
    };
    open-webui.enable = true;
  };
}

{pkgs ? import <nixpkgs> {}}:
pkgs.buildNpmPackage rec {
  pname = "ai";
  version = "1.2.3";

  src = pkgs.fetchFromGitHub {
    owner = "abhagsain";
    repo = "ai-cli";
    rev = "v${version}";
    sha256 = "137j00kx39p7g566ja0r78gg944k3s6i7p7327zmnpqgpxn4b5yl";
  };

  nativeBuildInputs = with pkgs; [
    installShellFiles
  ];

  npmDepsHash = "sha256-MR7Oqks9E8dCykhWjRhYOWFkK9ITeQlY1eCqfKMgbGM=";
  npmPackFlags = ["--ignore-scripts"];

  outputs = ["out"];

  # make it work
  postInstall = ''
    installShellCompletion --cmd ai \
      --bash <($out/bin/ai autocomplete:script bash) \
      --zsh <($out/bin/ai autocomplete:script zsh)
  '';

  meta = with pkgs.lib; {
    description = "Get answers for CLI commands from GPT3 right from your terminal";
    homepage = "https://github.com/abhagsain/ai-cli";
    license = licenses.gpl3Only;
    maintainers = with maintainers; [luisnquin];
  };
}

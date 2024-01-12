{pkgs, ...}: let
  inherit (pkgs) fetchFromGitHub;
in {
  tools.tplr = {
    enable = true;
    templates = {
      "go" = {
        "rest" = fetchFromGitHub rec {
          name = repo;
          owner = "luisnquin";
          repo = "go-rest-template";
          rev = "v0.1.0";
          sha256 = "0r09c22zpsiiy0rxdpkgg4ciq87x2hzq1ixqsjcfiqmmd6cx2c2n";
        };
        "cli" = builtins.path {
          name = "go-cli-template";
          path = ../../dots/project-templates/go/cli;
        };
      };
      "python" = {
        "cli" = builtins.path {
          name = "python-cli-template";
          path = ../../dots/project-templates/python/cli;
        };
      };
      "rust" = builtins.path {
        name = "rust-template";
        path = ../../dots/project-templates/rust;
      };
      "ocaml" = {
        "cli" = builtins.path {
          name = "ocaml-cli-template";
          path = ../../dots/project-templates/ocaml/cli;
        };
      };
    };
  };
}

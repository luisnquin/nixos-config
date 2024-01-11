{
  tools.tplr = {
    enable = true;
    templates = {
      "go" = {
        # "rest" = builtins.path {
        #   name = "go-rest-template";
        #   path = ../../dots/project-templates/go/rest;
        # };
        "cli" = builtins.path {
          name = "go-cli-template";
          path = ../../dots/project-templates/go/cli;
        };
      };
      "python" = {
        "cli" = builtins.path {
          name = "python-template";
          path = ../../dots/project-templates/python/cli;
        };
      };
      "rust" = builtins.path {
        name = "rust-template";
        path = ../../dots/project-templates/rust;
      };
    };
  };
}

{
  mkPoetryApplication,
  pkgs,
  ...
}:
mkPoetryApplication {
  python = pkgs.python310;
  projectDir = builtins.path {
    name = "poetry-app";
    path = ./.;
  };
  pyproject = builtins.path {
    name = "poetry-app-pyproject";
    path = ./pyproject.toml;
  };
  poetrylock = builtins.path {
    name = "poetry-app-poetrylock";
    path = ./poetry.lock;
  };
}

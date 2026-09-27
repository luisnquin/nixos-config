{
  lib,
  fetchFromGitHub,
  python3Packages,
}:
python3Packages.buildPythonApplication rec {
  pname = "davinci-resolve-mcp";
  version = "0.1.0-unstable-2026-04-07";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "apvlv";
    repo = "davinci-resolve-mcp";
    rev = "d025e4ecaf46f9ba7bcc1a814201791821c8f281";
    hash = "sha256-4yi0itkDYe6jG+qOsr/6hS+KdfHDVwXuB87zazJP7g8=";
  };

  postPatch = ''
    substituteInPlace pyproject.toml \
      --replace-fail '"uv_build>=0.9.21,<0.10.0"' '"uv_build>=0.9.21"'
    substituteInPlace src/davinci_resolve_mcp/resolve_api.py \
      --replace-fail 'resolve_script_dir = "/opt/resolve/Developer/Scripting"' \
        'resolve_script_dir = os.environ.get("RESOLVE_SCRIPT_API", "/opt/resolve/Developer/Scripting")'
  '';

  build-system = [python3Packages.uv-build];

  dependencies = with python3Packages; [
    mcp
    pydantic
    typing-extensions
  ];

  pythonImportsCheck = ["davinci_resolve_mcp"];

  meta = {
    description = "MCP server for DaVinci Resolve and Fusion";
    homepage = "https://github.com/apvlv/davinci-resolve-mcp";
    license = lib.licenses.mit;
    mainProgram = "davinci-resolve-mcp";
  };
}

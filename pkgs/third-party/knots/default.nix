{
  fetchFromGitHub,
  git,
  rustPlatform,
}:
rustPlatform.buildRustPackage rec {
  pname = "knots";
  version = "2.2.0";

  src = fetchFromGitHub {
    owner = "brandon-arrendondo";
    repo = "knots";
    tag = "v${version}";
    hash = "sha256-QSaXVYmEPPIez5UED/HXhzQXJ8VhVoSEB3tQrfCopI4=";
  };

  patches = [./staged.patch];

  nativeCheckInputs = [git];

  cargoHash = "sha256-SKTal1V0i1eAXbdP8NtDEBmkdmER/kklYWVLfGAzMV8=";

  meta.mainProgram = "knots";
}

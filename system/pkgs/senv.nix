{
  fetchFromGitHub,
  buildGoModule,
  lib,
}: let
  owner = "luisnquin";
  version = "0.4.2";
  commit = "d820302777c02bb622a2eed73a6ed8551d152205";
in
  buildGoModule rec {
    pname = "senv";
    inherit version;

    src = fetchFromGitHub {
      inherit owner;

      repo = pname;
      rev = "v${version}";
      sha256 = "sha256-xHtJi3AXceoLntSI3/I0rlHCzXyP4DT4f1UzddkzK2I=";
    };

    vendorSha256 = "sha256-C33Kj6PXoXa3OuH1ZP5kDJGR+BNaqbDrDGNtVpYgHZU=";
    doCheck = true;

    buildTarget = ".";
    ldflags = ["-X main.version=${version} -X main.commit=${commit}"];

    meta = with lib; {
      description = "Switch your .env file from the command line";
      homepage = "https://github.com/${owner}/${pname}";
      license = licenses.mit;
      maintainers = with maintainers; [luisnquin];
    };
  }

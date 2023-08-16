{
  fetchFromGitHub,
  buildGoModule,
  lib,
}: let
  owner = "luisnquin";
  version = "0.4.3";
  commit = "97642edf08b76d10dae9cbaf804e681f69749b02";
in
  buildGoModule rec {
    pname = "senv";
    inherit version;

    src = fetchFromGitHub {
      inherit owner;

      repo = pname;
      rev = "v${version}";
      sha256 = "sha256-lDfP8IoXvxwZu1iDDfzgsE/ko8ZJcSn9t2DM6p04fGY=";
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

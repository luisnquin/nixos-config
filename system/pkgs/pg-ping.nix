{
  buildGoModule,
  fetchFromGitHub,
  lib,
}: let
  owner = "luisnquin";
in
  buildGoModule rec {
    pname = "pg-ping";
    version = "1.1.0";
    src = fetchFromGitHub {
      inherit owner;

      repo = pname;
      rev = "3d083e562cc48a55111c7558d65d6bc66cde9f2a";
      sha256 = "0p0fvg82zjj0jgh0yv8nbxkm7lb99f0p8w17kb00rwadrzh4la2y";
    };

    ldflags = ["-X main.version=${version}"];
    buildTarget = "./";

    vendorSha256 = "sha256-8/6pJtJ1h1miV1SxMP9l9+ckWrCl8QjrK6M8lUx06KY=";
    doCheck = false;

    meta = with lib; {
      description = "Ping your postgres continuously";
      homepage = "https://github.com/${owner}/${pname}";
      # license = licenses.mit;
      maintainers = with maintainers; ["${owner}"];
    };
  }

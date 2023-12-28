{
  buildGoModule,
  fetchFromGitHub,
  lib,
}: let
  owner = "luisnquin";
in
  buildGoModule rec {
    pname = "no";
    version = "0.0.1";
    src = fetchFromGitHub {
      inherit owner;
      repo = pname;
      rev = "e466d78346426db9f30eb947d015040c7b2ded88";
      sha256 = "sha256-npgmw1eucEtd8O4PYeOgd4PhuX+JX6FGue5tKQOep7A=";
    };

    ldflags = ["-X main.version=${version}"];
    buildTarget = ".";

    vendorHash = "sha256-bGDhws+Ye/VDqIAcfBTIkjfp3IWkEx9a/fwri0wl258=";
    doCheck = false;

    meta = with lib; {
      description = "Unlike gnu yes";
      homepage = "https://github.com/${owner}/${pname}";
      license = licenses.gpl3Only;
      maintainers = with maintainers; ["${owner}"];
    };
  }

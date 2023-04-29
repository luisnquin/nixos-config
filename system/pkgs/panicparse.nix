{pkgs ? import <nixpkgs> {}}: let
  owner = "maruel";
in
  pkgs.buildGoModule rec {
    pname = "pp";
    version = "2.3.1";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = "panicparse";
      rev = "a67acbb1be08722cbfb23fcfff41ed435b9fd329";
      sha256 = "156mawfrq1i43sxkvy4ci5hx6bxaw12z4dgpykj2fq1xq7ykhn19";
    };

    ldflags = ["-X main.version=${version}"];

    buildTarget = "./cmd/pp";

    vendorSha256 = "sha256-8sUW2cTlo5z4fmAezK9Gz5JqJRH1nf3QOVI5+UlV00s=";
    doCheck = false;

    meta = with pkgs.lib; {
      description = "Crash your app in style";
      homepage = "https://github.com/${owner}/${pname}";
      license = licenses.asl20;
      maintainers = with maintainers; [luisnquin];
    };
  }

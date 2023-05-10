{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.buildGoModule rec {
    pname = "pg-ping";
    version = "1.1.0";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = pname;
      rev = "7670745339e535a644d7592e5c7b3fea13fddf8f";
      sha256 = "0y34q5g6ngxlwr40var6h6rqi47njqz07yss1ph1iw4bsaja9kgx";
    };

    ldflags = ["-X main.version=${version}"];
    buildTarget = "./";

    vendorSha256 = "sha256-8/6pJtJ1h1miV1SxMP9l9+ckWrCl8QjrK6M8lUx06KY=";
    doCheck = false;

    meta = with pkgs.lib; {
      description = "Ping your postgres continuously";
      homepage = "https://github.com/${owner}/${pname}";
      # license = licenses.mit;
      maintainers = with maintainers; ["${owner}"];
    };
  }

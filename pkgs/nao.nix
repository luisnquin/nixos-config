{
  lib,
  buildGoModule,
  fetchFromGitHub,
  ...
}
: let
  owner = "luisnquin";
in
  buildGoModule rec {
    pname = "nao";
    version = "3.0.0";
    src = fetchFromGitHub {
      owner = owner;
      repo = pname;
      rev = "v${version}";
      # sha256 = "0m2fzpqxk7hrbxsgqplkg7h2p7gv6s1miymv3gvw0cz039skag0s";
    };

    ldflags = ["-X main.version=${version}"];
    buildTarget = "./cmd/nao";

    vendorSha256 = "";
    doCheck = false;

    meta = with lib; {
      description = "A CLI tool to take notes without worrying about the path where the file is";
      homepage = "https://github.com/${owner}/${pname}";
      license = licenses.mit;
      maintainers = with maintainers; [${owner}];
    };
  }

{
  buildGoModule,
  fetchFromGitHub,
  lib,
}
: let
  owner = "luisnquin";
in
  buildGoModule rec {
    pname = "nao";
    version = "3.2.2";
    src = fetchFromGitHub {
      inherit owner;
      repo = pname;
      rev = "v${version}";
      sha256 = "sha256-hB5AqlFVYYjTx10v6jc45+IT0X/f0tLs1S8V68uhwYs=";
    };

    vendorSha256 = "sha256-MTVJWksGWva+Xet+T2aIOXzkxB7w9raJVwa/p1bwkOo=";
    doCheck = false;

    buildTarget = "./cmd/nao";
    ldflags = ["-X main.version=${version}"];

    # nativeBuildInputs = with pkgs; [installShellFiles];

    # postInstall = ''
    #   installShellCompletion --cmd nao \
    #     --bash <($out/bin/nao completion bash) \
    #     --fish <($out/bin/nao completion fish) \
    #     --zsh <($out/bin/nao completion zsh)
    # '';

    meta = with lib; {
      description = "A CLI tool to take notes without worrying about the path where the file is";
      homepage = "https://github.com/${owner}/${pname}";
      license = licenses.mit;
      maintainers = with maintainers; ["${owner}"];
    };
  }

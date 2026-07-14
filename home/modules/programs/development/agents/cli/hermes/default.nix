{
  lib,
  pkgs,
  ...
}: let
  hermes = pkgs.hermes-agent.overrideAttrs (old: {
    postInstall =
      (old.postInstall or "")
      + ''
        for program in hermes hermes-agent hermes-acp; do
          wrapProgram "$out/bin/$program" \
            --set GIT_SSH_COMMAND "${lib.getExe pkgs.openssh} -o BatchMode=yes"
        done
      '';
  });
in {
  home.packages = [hermes];
}

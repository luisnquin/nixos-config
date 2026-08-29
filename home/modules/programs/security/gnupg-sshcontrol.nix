{
  # gpg-agent runs system-side; services.gpg-agent.sshKeys would start a second agent.
  home.file.".gnupg/sshcontrol" = {
    force = true;
    text = ''
      # luis@quinones.pro -- ~/.ssh/id_ed25519
      8A6101143405A9A8E27EAB40FBCE7D1EEC516C8D 0
    '';
  };
}

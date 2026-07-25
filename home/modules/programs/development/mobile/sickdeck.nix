{
  # The service, simulators and emulators live on rose; this host is a remote
  # client reached over the tailnet. `rose` resolves via MagicDNS. The skill is
  # published by the agents module, not here, so skill.enable stays off.
  programs.sickdeck = {
    enable = true;
    remote.url = "http://rose:4310";
  };
}

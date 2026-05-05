let
  luisnquin = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIXW6vsDRgI/AiOdGnQOTyiz1uLFL0o66u0Ahcw9VWyd";

  defaultKeys = [luisnquin];
in {
  "certs/ccd/rootCA.crt.age".publicKeys = defaultKeys;
  "certs/ccd/rootCA.key.age".publicKeys = defaultKeys;
  "certs/ccd/wildcard.crt.age".publicKeys = defaultKeys;
  "certs/ccd/wildcard.key.age".publicKeys = defaultKeys;
  "tailscale/auth-key.age".publicKeys = defaultKeys;
}

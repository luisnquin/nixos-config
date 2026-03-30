let
  luisnquin = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIXW6vsDRgI/AiOdGnQOTyiz1uLFL0o66u0Ahcw9VWyd luis@quinones.pro";
in
{
  "secrets/certs/rootCA.crt.age".publicKeys = [luisnquin];
  "secrets/certs/rootCA.key.age".publicKeys = [luisnquin];
  "secrets/certs/wildcard.crt.age".publicKeys = [luisnquin];
  "secrets/certs/wildcard.key.age".publicKeys = [luisnquin];
}

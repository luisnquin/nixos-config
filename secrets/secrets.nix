let
  luisnquin = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIXW6vsDRgI/AiOdGnQOTyiz1uLFL0o66u0Ahcw9VWyd luis@quinones.pro";
in {
  "tailscale/auth-key.age".publicKeys = [luisnquin];
}

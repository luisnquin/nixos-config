let
  luisnquin = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPgB5sEmG7PDxQ8D240uM2uSJKUPXmY/zMxtfr2jf7Pl lpaandres2020@gmail.com";
in {
  "spotify-access-secret.age".publicKeys = [luisnquin];
  "40a98b0e-fca3-42a4-8867-c453a33d45e6.age".publicKeys = [luisnquin];
}

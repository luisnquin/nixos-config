# This file by default is chmod 600, so only the owners
# can read/write this file
let
  secrets = rec {
    spotifyPassword = "<password-here>";
  };
in
  secrets

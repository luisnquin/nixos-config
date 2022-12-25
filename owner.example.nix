let
  owner = rec {
    username = "<your-username>";
    fullName = "<your-full-name>";
    email = "<your-email>";
    secondaryEmail = "<your-secondary-email>";

    git = {
      inherit username;
      name = fullName;
      email = "<your-git-email>";
      deprecatedEmail = secondaryEmail;
    };

    spotifyUsername = "<your-spotify-username>";
  };

  secrets = import "/etc/nixos/secrets.nix";
in
  owner // secrets

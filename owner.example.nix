let
  owner = rec {
    username = "<username>";
    fullName = "<full-name>";
    email = "<email>";

    git = {
      inherit username;
      name = fullName;
      email = "<git-email>";
    };
  };
in
  owner

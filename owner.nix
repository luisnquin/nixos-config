let
  owner = rec {
    username = "luisnquin";
    email = "root@luisquinones.me";

    git = rec {
      inherit username;
      email = "git@luisquinones.me";
    };

    spotify = rec {
      username = "yeselony";
    };
  };
in
  owner

let
  owner = rec {
    username = "luisnquin";
    fullName = "Luis Quiñones Requelme";

    email = "root@luisquinones.me";
    secondaryEmail = "lpaandres2020@gmail.com";

    git = {
      inherit username;
      name = fullName;

      email = "git@luisquinones.me";
      deprecatedEmail = secondaryEmail;
    };
  };
in
  owner

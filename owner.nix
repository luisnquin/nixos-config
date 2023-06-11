let
  owner = rec {
    username = "luisnquin";
    fullName = "Luis Quiñones Requelme";
    email = "lpaandres2020@gmail.com";

    git = {
      inherit username;
      name = fullName;
      email = "git@luisquinones.me";
    };
  };
in
  owner

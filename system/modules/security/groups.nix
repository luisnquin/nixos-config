{user, ...}: {
  users.groups = {
    vboxusers.members = [user.alias];

    control = {
      name = "pain-control";
      members = [user.alias];
    };
  };
}

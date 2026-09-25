{
  config,
  user,
  lib,
  ...
}: {
  users.groups = {
    vboxusers.members = [user.alias];

    nginx = lib.mkForce {
      gid = config.ids.gids.nginx;
    };
  };
}

{
  pkgs,
  user,
  ...
}: {
  services.incron = {
    enable = true;

    # Packages available to the system incrontab.
    extraPackages = [pkgs.coreutils];

    systab = let
      homeDirectory = "/home/${user.alias}";
    in ''
      ${homeDirectory} IN_CREATE rm -f ${homeDirectory}/discord_{krisp,utils}.log $#
    '';
  };
}

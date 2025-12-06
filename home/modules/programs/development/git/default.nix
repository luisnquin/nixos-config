{user, ...}: {
  imports = [
    ./terminal.nix
    ./jujutsu.nix
    ./github.nix
    ./etc.nix
  ];

  shared = {
    git = {
      enable = true;
      user = {
        name = user.fullName;
        email = user.gitEmail;
      };
    };
    lazygit.enable = true;
  };
}

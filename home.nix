{pkgs, ...}: {
  programs.git = {
    enable = true;
    userName = "luisnquin";
    userEmail = "lpaandres2020@gmail.com";
  };

  programs.bash = {
    enable = true;
  };

  programs.go.enable = true;
}

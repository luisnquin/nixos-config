{mysql_57, ...}: {
  # by default (user: root) + no password
  services.mysql = {
    enable = false;
    package = mysql_57;
  };

  environment.systemPackages = [
    mysql_57
  ];
}

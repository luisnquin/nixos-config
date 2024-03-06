{mysql_57, ...}: {
  # by default (user: root) + no password
  services.mysql = {
    enable = true;
    package = mysql_57;
  };
}

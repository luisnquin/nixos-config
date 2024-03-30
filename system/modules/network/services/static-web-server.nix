{inner-static, ...}: {
  services.static-web-server = {
    enable = true;
    listen = "[::]:8787";

    root = "${inner-static}/var/www/html";
  };
}

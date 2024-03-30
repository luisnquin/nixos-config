{inner-static, ...}: {
  services.static-web-server = {
    enable = true;
    listen = "[::]:8787";

    root = "${inner-static}/var/www/html/root";

    configuration = {
      general = {
        page404 = "${inner-static}/var/www/html/404.html";
        page50x = "${inner-static}/var/www/html/50x.html";

        log-level = "warn";

        cache-control-headers = true;
        compression = true;

        https-redirect = false; # managed with nginx

        directory-listing = true;
        directory-listing-order = 1;
        directory-listing-format = "html";

        health = false;
        maintenance-mode = false;
      };
    };
  };
}

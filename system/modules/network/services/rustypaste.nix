{user, ...}: {
  services.rustypaste = {
    enable = true;

    settings = {
      config = {
        refresh_rate = "1s";
      };

      server = {
        address = "127.0.0.1:8000";
        #url = "https://rustypaste.shuttleapp.rs"
        #workers=4
        max_content_length = "10MB";
        upload_path = "/home/${user.alias}/.local/share/rustypaste/upload";
        timeout = "30s";
        expose_version = false;
        expose_list = true;
        handle_spaces = "replace"; # or "encode"
        #auth_tokens = [
        #  "super_secret_token1",
        #  "super_secret_token2",
        #]
        #delete_tokens = [
        #  "super_secret_token1",
        #  "super_secret_token3",
        #]
      };

      landing_page = {
        text = builtins.readFile ./dots/landing.txt;
        #file = "index.txt"
        content_type = "text/plain; charset=utf-8";
      };

      paste = {
        random_url = {
          type = "petname";
          words = 2;
          separator = "-";
        };
        #random_url = { type = "alphanumeric", length = 8 }
        #random_url = { type = "alphanumeric", length = 6, suffix_mode = true }
        default_extension = "txt";
        mime_override = [
          {
            mime = "image/jpeg";
            regex = "^.*\\.jpg$";
          }
          {
            mime = "image/png";
            regex = "^.*\\.png$";
          }
          {
            mime = "image/svg+xml";
            regex = "^.*\\.svg$";
          }
          {
            mime = "video/webm";
            regex = "^.*\\.webm$";
          }
          {
            mime = "video/x-matroska";
            regex = "^.*\\.mkv$";
          }
          {
            mime = "application/octet-stream";
            regex = "^.*\\.bin$";
          }
          {
            mime = "text/plain";
            regex = "^.*\\.(log|txt|diff|sh|rs|toml)$";
          }
        ];
        mime_blacklist = [
          "application/x-dosexec"
          "application/java-archive"
          "application/java-vm"
        ];
        duplicate_files = true;
        # default_expiry = "1h"
        delete_expired_files = {
          enabled = true;
          interval = "1h";
        };
      };
    };
  };
}

{
  services.gnome-keyring = {
    enable = true;
    components = ["secrets" "pkcs11"];
  };

  home.file.".local/share/keyrings/default" = {
    text = "login\n";
    force = true;
  };
}

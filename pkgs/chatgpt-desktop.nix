{pkgs ? import <nixpkgs> {}}:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "chatgpt-desktop";
  version = "0.0.1";

  src = pkgs.fetchFromGitHub {
    owner = "luisnquin";
    repo = pname;
    sha256 = "1bv1wpyhn0vnvamhizz351pzy56lhrd2wd2cdsf7fyaw90rgpb7d";
    rev = "e51723190f326fdc0853b26a5b8628b88eabe209";
  };

  sourceRoot = "source/src-tauri";

  cargoSha256 = "sha256-QKChboUfJg+VFDYQF2vCZZLLUpVI6yFkRh2wJyuFLXQ=";

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    webkitgtk
    openssl
    libsoup
    dbus
    gtk3
    glib
  ];

  propagatedBuildInputs = with pkgs; [
    libayatana-appindicator
    glib-networking
  ];

  # shellHook = with pkgs; ''
  #   export LD_LIBRARY_PATH="${libayatana-appindicator}/lib/"
  #   export GIO_MODULE_DIR=${glib-networking}/lib/gio/modules/
  # '';

  meta = with pkgs.lib; {
    description = "OpenAI ChatGPT desktop app for Mac, Windows, & Linux menubar using Tauri & Rust";
    homepage = "https://github.com/sonnylazuardi/${pname}";
    license = licenses.mit;
    maintainers = ["luisnquin"];
  };
}
# export GIO_MODULE_DIR="/nix/store/z4g7hin59rsav6i986ha2cnm8w1bw6wy-glib-networking-2.74.0/lib/gio/modules"
# export LD_LIBRARY_PATH="/nix/store/fnm8qxcxl6ilryc2dy0d5np8kih6y4f2-libayatana-appindicator-0.5.91/lib"
#
# Could not determine the accessibility bus address
# GStreamer element appsink not found. Please install it


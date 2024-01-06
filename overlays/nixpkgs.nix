[
  (self: super: {
    ranger =
      super.ranger.overrideAttrs
      (r: {
        preConfigure =
          r.preConfigure
          + ''
            substituteInPlace ranger/ext/img_display.py \
              --replace "Popen(['ueberzug'" \
                        "Popen(['${self.ueberzugpp}/bin/ueberzugpp'"

            substituteInPlace ranger/config/rc.conf \
              --replace 'set preview_images_method w3m' \
                        'set preview_images_method ueberzug'
          '';
      });
  })
  (self: super: {
    xwaylandvideobridge =
      super.xwaylandvideobridge.overrideAttrs
      (old: {
        version = "unstable";
        src = self.fetchFromGitHub {
          owner = "KDE";
          repo = "xwaylandvideobridge";
          rev = "6589a2c5f9eb952fb501635c0d9870ec1abebb10";
          hash = "sha256-cOxmLE5LJItUaqQCJaDBzNjyRGmyFO0NFTV/CpzmYco=";
        };
      });
  })
]

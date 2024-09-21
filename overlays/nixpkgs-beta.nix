[
  (
    self: super: {
      ranger =
        super.ranger.overrideAttrs
        (r: {
          propagatedBuildInputs = r.propagatedBuildInputs ++ [self.ueberzugpp];

          preConfigure =
            r.preConfigure
            + ''
              substituteInPlace ranger/ext/img_display.py \
                --replace-fail '"ueberzug"' '"ueberzugpp"'

                substituteInPlace ranger/config/rc.conf \
                  --replace-fail 'set preview_images_method w3m' 'set preview_images_method ueberzugpp'
            '';
        });
    }
  )
]

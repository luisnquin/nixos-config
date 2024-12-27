[
  (
    _self: super: {
      logkeys = super.logkeys.overrideAttrs (prev: {
        postPatch =
          prev.postPatch
          + ''
            mkdir -p $out/etc/logkeys/keymaps
            cp $src/keymaps/es_ES.map $out/etc/logkeys/keymaps

            substituteInPlace scripts/logkeys-start.sh \
              --replace 'logkeys --start' 'logkeys --keymap $out/etc/logkeys/keymaps/es_ES.map --start'
          '';
      });
    }
  )
  (
    _self: super: {
      panicparse = super.panicparse.overrideAttrs (_old: {
        postInstall = ''
          mv $out/bin/panicparse $out/bin/pp
        '';
      });
    }
  )
]

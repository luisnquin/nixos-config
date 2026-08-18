{
  lib,
  rustPlatform,
  pkg-config,
  makeWrapper,
  writeText,
  wayland,
  libxkbcommon,
  fontconfig,
  freetype,
  cascadia-code,
  dejavu_fonts,
  # First entry drives the interface font (see `defaultFont`); the rest widen
  # preview coverage. A small set bounds the startup font scan.
  fonts ? [cascadia-code dejavu_fonts],
  # Empty derives the interface font family from the first `fonts` entry via
  # fc-scan. Override when that package ships several families.
  defaultFont ? "",
}: let
  # fontdb replaces (does not merge) the system fontconfig when FONTCONFIG_FILE
  # is set, so this limits the startup scan to `fonts`.
  fontsConf = writeText "cliplenz-fonts.conf" ''
    <?xml version="1.0"?>
    <!DOCTYPE fontconfig SYSTEM "fonts.dtd">
    <fontconfig>
    ${lib.concatMapStringsSep "\n" (f: "  <dir>${f}/share/fonts</dir>") fonts}
    </fontconfig>
  '';
in
  rustPlatform.buildRustPackage {
    pname = "cliplenz";
    version = "0.1.0";

    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
      ];
    };

    cargoLock.lockFile = ./Cargo.lock;

    # fontconfig provides fc-scan, used below to derive the interface font.
    nativeBuildInputs = [pkg-config makeWrapper fontconfig];

    buildInputs = [wayland libxkbcommon fontconfig freetype];

    # tiny-skia forces the software renderer; the LD_LIBRARY_PATH prefix lets the
    # binary run without an ambient one.
    postInstall = ''
      font=${lib.escapeShellArg defaultFont}
      ${lib.optionalString (fonts != []) ''
        if [ -z "$font" ]; then
          font=$(fc-scan --format '%{family[0]}\n' ${lib.escapeShellArg "${builtins.head fonts}/share/fonts"} | sort -u | head -n1)
        fi
      ''}
      fontArgs=()
      if [ -n "$font" ]; then fontArgs+=(--set CLIPLENZ_FONT "$font"); fi
      wrapProgram $out/bin/cliplenz \
        --set ICED_BACKEND tiny-skia \
        --set FONTCONFIG_FILE ${fontsConf} \
        "''${fontArgs[@]}" \
        --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [wayland libxkbcommon fontconfig freetype]}
    '';

    meta = {
      description = "Fast native dmenu with fuzzy search and image preview";
      license = lib.licenses.mit;
      mainProgram = "cliplenz";
      platforms = lib.platforms.linux;
    };
  }

{
  lib,
  stdenv,
  callPackage,
  rustPlatform,
  installShellFiles,
  makeWrapper,
  android-tools,
  avahi,
  ffmpeg-headless,
  fzf,
  libnotify,
  openssh,
  scrcpy,
  tailscale,
  wl-clipboard,
}: let
  # Split by hand rather than filtered on `lib.meta.availableOn`, which only
  # reads the top package's own platform list: wl-clipboard claims unix and then
  # fails on a wayland it can only have here.
  runtimeInputs =
    [
      android-tools
      # `record --frames` cuts the clip here, not on the host that holds
      # the device: a mac running a simulator is not required to have ffmpeg
      ffmpeg-headless
      fzf
      openssh
      scrcpy
      tailscale
    ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [
      # mDNS, desktop notifications and the clipboard: macOS answers all three
      # with something already in the OS, so the commands that reach for these
      # are not the ones that run there.
      avahi
      libnotify
      wl-clipboard
    ];
in
  rustPlatform.buildRustPackage {
    pname = "phone";
    version = "0.1.0";

    # only the crate itself, so editing this file does not rebuild it
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
      ];
    };

    cargoLock.lockFile = ./Cargo.lock;

    nativeBuildInputs = [installShellFiles makeWrapper];

    postInstall = ''
      installShellCompletion --cmd phone \
        --bash <($out/bin/phone completions bash) \
        --fish <($out/bin/phone completions fish) \
        --zsh <($out/bin/phone completions zsh)

      wrapProgram $out/bin/phone \
        --prefix PATH : ${lib.makeBinPath runtimeInputs}
    '';

    # The far side of the ssh link: the accessibility bridge, which only builds
    # on darwin. Kept in passthru rather than as an input so this derivation does
    # not carry a mac-only dependency on linux.
    passthru.receiver = callPackage ./receiver {};

    meta.mainProgram = "phone";
  }

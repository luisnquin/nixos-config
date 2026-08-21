{
  lib,
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
  runtimeInputs = [
    android-tools
    avahi
    # `record --frames` cuts the clip here, not on the host that holds
    # the device: a mac running a simulator is not required to have ffmpeg
    ffmpeg-headless
    fzf
    libnotify
    openssh
    scrcpy
    tailscale
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

    # Reachable on darwin, where this CLI's own runtime inputs are not:
    # passthru is not forced with the derivation.
    passthru.receiver = callPackage ./receiver {};

    meta.mainProgram = "phone";
  }

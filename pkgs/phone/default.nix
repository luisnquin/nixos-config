{
  lib,
  rustPlatform,
  installShellFiles,
  makeWrapper,
  android-tools,
  avahi,
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

    meta.mainProgram = "phone";
  }

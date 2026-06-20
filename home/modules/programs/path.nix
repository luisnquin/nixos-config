{config, ...}: {
  home.sessionPath = [
    "${config.home.homeDirectory}/.cargo/bin"
    "${config.home.homeDirectory}/.npm-global/bin"
    "${config.home.homeDirectory}/.bun-global/bin"
    "${config.home.homeDirectory}/go/bin"
    "${config.home.homeDirectory}/.android/platform-tools"
    "${config.home.homeDirectory}/.android/emulator"

    "${config.home.homeDirectory}/.local/share/flatpak/exports/bin"
  ];
}

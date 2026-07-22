{pkgs, ...}: {
  home.packages = [
    # Chromium derives the password store from the desktop environment, and
    # Hyprland falls through to basic_text, leaving Electron's safeStorage
    # unavailable so the app drops the session right after signing in.
    (pkgs.symlinkJoin {
      name = "claude-desktop-gnome-libsecret";
      paths = [pkgs.llm-agents.claude-desktop];
      nativeBuildInputs = [pkgs.makeWrapper];
      postBuild = ''
        wrapProgram $out/bin/claude-desktop --add-flags "--password-store=gnome-libsecret"
      '';
    })
  ];
}

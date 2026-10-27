{pkgs, ...}: {
  home.packages = [
    pkgs.davinci-resolve
    pkgs.shotcut
    pkgs.ffmpeg
  ];

  xdg.desktopEntries.davinci-resolve = {
    name = "Davinci Resolve";
    genericName = "Video Editor";
    comment = "Professional video editing, color, effects and audio post-processing";
    exec = "davinci-resolve";
    icon = "davinci-resolve";
    categories = ["AudioVideo" "AudioVideoEditing" "Video" "Graphics"];
    settings = {
      StartupWMClass = "resolve";
      PrefersNonDefaultGPU = "true";
    };
  };

  programs.obs-studio = {
    enable = true;

    plugins = with pkgs; [
      obs-studio-plugins.wlrobs
    ];
  };
}

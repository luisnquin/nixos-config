{user, ...}: {
  xdg.configFile = {
    "greenclip.toml".text = ''
      [greenclip]
      history_file = "/home/${user.alias}/.cache/greenclip.history"
      max_history_length = 50
      max_selection_size_bytes = 0
      trim_space_from_selection = true
      use_primary_selection_as_input = false
      blacklisted_applications = []
      enable_image_support = true
      image_cache_directory = "/tmp/greenclip"
      static_history = ["what's the use of this"]
    '';
  };
}

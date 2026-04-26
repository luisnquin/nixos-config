{
  pkgs,
  config,
  ...
}: {
  programs.fuzzel = {
    enable = true;
    settings = {
      main = {
        terminal = "${pkgs.lib.getExe config.programs.ghostty.package}";
        layer = "overlay";
        font = "Cascadia Code:size=9";
        prompt = "❯ ";
        icons-enabled = "no";
        show-actions = "no";
        lines = 10;
        width = 40;
        horizontal-pad = 32;
        vertical-pad = 8;
        inner-pad = 8;
        line-height = 20;
        letter-spacing = 0;
        match-mode = "fzf";
        anchor = "center";
        x-margin = 0;
        y-margin = 0;
      };

      colors = {
        background = "1e1e2eff";
        text = "cdd6f4ff";
        match = "f38ba8ff";
        selection = "585b70ff";
        selection-match = "f38ba8ff";
        selection-text = "cdd6f4ff";
        border = "b4befeff";
      };

      border = {
        width = 2;
        radius = 8;
      };

      dmenu = {
        mode = "text";
        exit-immediately-if-empty = "yes";
      };

      key-bindings = {
        cancel = "Escape Control+g Control+c";
        execute = "Return KP_Enter Control+y";
        execute-or-next = "Tab";
        prev = "Up Control+p";
        next = "Down Control+n";
        delete-line-backward = "Control+u";
        delete-line-forward = "Control+k";
        delete-prev-word = "Control+BackSpace Control+w";
      };
    };
  };
}

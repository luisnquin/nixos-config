{
  programs.opencode = {
    enable = true;

    settings = {
      server.port = 4096;

      permission = {
        read = "allow";
        glob = "allow";
        grep = "allow";
        list = "allow";
        lsp = "allow";

        edit = "allow";
        write = "allow";

        bash = "ask";

        webfetch = "allow";
        websearch = "ask";
      };

      snapshot = true;
      autoupdate = false;
    };
  };
}

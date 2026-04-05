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

      model = "litellm/gemma4:e4b";

      provider = {
        litellm = {
          npm = "@ai-sdk/openai-compatible";
          name = "LiteLLM";
          options = {
            "baseURL" = "http://dyx.local:4000/v1";
            "apiKey" = "dummy";
          };
          models = {
            "gemma4:e4b" = {
              "name" = "Gemma 4";
            };
          };
        };
      };

      snapshot = true;
      autoupdate = false;
    };
  };
}

{
  pkgs,
  mkAgentKit,
  ...
}: let
  kit = mkAgentKit {};
in {
  # https://opencode.ai/config.json
  programs.opencode = {
    enable = true;
    package = pkgs.llm-agents.opencode;

    plugins = {
      "rtk" = "${pkgs.rtk}/share/rtk/hooks/opencode-rtk.ts";
    };

    settings = {
      server.port = 4096;

      permission = kit.mkAgentPermissions "opencode";

      model = "litellm/qwen2.5-coder:7b";

      provider = {
        litellm = {
          npm = "@ai-sdk/openai-compatible";
          name = "LiteLLM";
          options = {
            "baseURL" = "http://rose.local:4000/v1";
            "apiKey" = "dummy";
          };
          models = {
            "gemma4:e4b" = {
              "name" = "Gemma 4";
            };
            "qwen2.5-coder:7b" = {
              "name" = "Qwen 2.5 - Coder";
            };
          };
        };
      };

      snapshot = true;
      autoupdate = false;
    };
  };
}

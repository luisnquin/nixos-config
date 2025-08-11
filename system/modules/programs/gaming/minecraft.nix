{
  inputs,
  pkgs,
  ...
}: {
  environment.systemPackages = [
    inputs.prismlauncher.packages.${pkgs.system}.default # jailbreak offline accounts
  ];

  services.flatpak.packages = [
    {
      # Minecraft Bedrock (Play Store)
      appId = "io.mrarm.mcpelauncher";
      origin = "flathub";
    }
  ];
}

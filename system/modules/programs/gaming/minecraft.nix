{libx, ...}: {
  # environment.systemPackages = [
  #   inputs.prismlauncher.packages.${pkgs.stdenv.hostPlatform.system}.default # jailbreak offline accounts
  # ];

  services.flatpak.packages = [
    {
      # Minecraft Bedrock (Play Store)
      appId = "io.mrarm.mcpelauncher";
      origin = "flathub";
    }
  ];

  services.flatpak.overrides."io.mrarm.mcpelauncher".Environment = libx.nvidia.offloadEnv;
}

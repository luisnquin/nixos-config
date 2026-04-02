{pkgs, ...}: {
  home = {
    packages = [pkgs.rtk];
    sessionVariables = {
      "RTK_TELEMETRY_DISABLED" = "1";
    };
  };
}

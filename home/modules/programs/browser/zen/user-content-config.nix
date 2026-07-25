{
  pkgs,
  lib,
  ...
}: let
  # user-origin sheet: page rules outrank it unless every declaration is forced.
  forceImportant = name: src:
    pkgs.runCommand "zen-site-${name}.css" {} ''
      sed -E '/!important/! s/:([^;{}]+);/:\1 !important;/g' ${src} > $out
    '';

  siteStyles = [
    (forceImportant "hackers-new-css" (pkgs.fetchurl {
      url = "https://gist.githubusercontent.com/christippett/5097af0ea59c867c4578996350933776/raw/fe63eca0b5e013a685ed16b408b619fa2a0af4d7/hn.user.css";
      hash = "sha256-iwWKJRlXylHPRlm4wLhRx2ZPscogRnfwVdCgEMXc/Ng=";
    }))
  ];
in
  pkgs.runCommand "zen-usercontent.css" {} ''
    cat ${lib.concatStringsSep " " siteStyles} > $out
  ''

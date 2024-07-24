{pkgs, ...}: {
  home.packages = with pkgs; [
    exiftool
    # maigret # Broken -> ValueError: Hash algorithm not known for ed448 - use .cms_hash_algorithm for CMS purposes. More info at https://github.com/wbond/asn1crypto/pull/230.
    whois
  ];
}

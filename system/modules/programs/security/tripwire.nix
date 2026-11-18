{
  config,
  lib,
  pkgs,
  user,
  ...
}: {
  services.tripwire = {
    enable = true;
    user = user.alias;
    decoys = [
      ".git-credentials"
      ".ssh/aws_prod.pem"
      ".electrum/wallets/default_wallet"
      ".config/solana/id.json"
      ".config/BraveSoftware/Brave-Browser/Local State"
      ".config/BraveSoftware/Brave-Browser/Default/Login Data"
      ".config/BraveSoftware/Brave-Browser/Default/Cookies"
      ".config/microsoft-edge/Default/Login Data"
    ];
    extraAlert = ''
      ${lib.getExe pkgs.curl} -fsS -m 10 -H "Priority: 5" -H "Tags: rotating_light" \
        -H "Title: ${config.networking.hostName} tripwire" \
        -d "Decoy $TRIPWIRE_EVENT: $TRIPWIRE_PATH $TRIPWIRE_FROZEN" \
        ${config.services.ntfy-sh.settings.base-url}/nyx >/dev/null
    '';
  };
}

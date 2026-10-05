{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) filterAttrs mapAttrs mkEnableOption mkIf mkOption optionalAttrs types;
  cfg = config.programs.phone;

  toml = pkgs.formats.toml {};

  ttl = mkOption {
    type = types.nullOr types.str;
    default = null;
    example = "20m";
    description = "How long a device stays held after its holder's last verb.";
  };

  device = types.submodule {
    options = {
      kind = mkOption {
        type = types.nullOr (types.enum ["physical" "virtual"]);
        default = null;
        description = "Overrides what the platform implies; a physical device is watched for human touch.";
      };

      pick = mkOption {
        type = types.enum ["first" "normal" "last" "never"];
        default = "normal";
        description = "Where the allocator ranks the device; never means only an explicit target reaches it.";
      };

      lease.ttl = ttl;
    };
  };

  host = types.submodule {
    options.clone = mkOption {
      type = types.bool;
      default = false;
      description = "Whether the allocator may clone an AVD on this host once the pool is exhausted.";
    };
  };

  leaseOf = l: optionalAttrs (l.ttl != null) {lease.ttl = l.ttl;};

  rendered =
    filterAttrs (_: v: v != {}) {
      lease = optionalAttrs (cfg.lease.ttl != null) {inherit (cfg.lease) ttl;};
      inherit (cfg) hosts pools;
      devices =
        mapAttrs (
          _: d:
            {inherit (d) pick;}
            // optionalAttrs (d.kind != null) {inherit (d) kind;}
            // leaseOf d.lease
        )
        cfg.devices;
    };
in {
  options.programs.phone = {
    enable = mkEnableOption "the phone device CLI";

    package = mkOption {
      type = types.package;
      default = pkgs.phone;
      description = "The phone package.";
    };

    lease.ttl = ttl;

    hosts = mkOption {
      type = types.attrsOf host;
      default = {};
      description = "Per ssh host settings.";
    };

    pools = mkOption {
      type = types.attrsOf (types.listOf types.str);
      default = {};
      example = {android = ["pixel_7-api36" "pixel_7-api36-b"];};
      description = "Devices an untargeted verb may be handed, per platform. A platform with no pool hands out any of its devices.";
    };

    devices = mkOption {
      type = types.attrsOf device;
      default = {};
      description = "Per device settings, keyed by AVD name or label.";
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile."phone/config.toml".source = toml.generate "phone-config.toml" rendered;
  };
}

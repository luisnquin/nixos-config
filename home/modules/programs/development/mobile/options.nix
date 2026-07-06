{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit
    (lib)
    concatStringsSep
    filterAttrs
    mapAttrs'
    mapAttrsToList
    mkIf
    mkOption
    nameValuePair
    optionalAttrs
    types
    ;

  cfg = config.android.avds;
  enabledAvds = filterAttrs (_: avd: avd.enable) cfg;

  androidHome = "${config.home.homeDirectory}/.android";
  avdRoot = "${androidHome}/avd";
  skinRoot = "${config.home.homeDirectory}/.config/android/skins";

  boolToIni = value:
    if value
    then "yes"
    else "no";

  valueToIni = value:
    if lib.isBool value
    then boolToIni value
    else toString value;

  mkIni = attrs:
    concatStringsSep "\n"
    (mapAttrsToList (key: value: "${key}=${valueToIni value}") attrs)
    + "\n";

  mkTagDisplay = systemImage:
    if systemImage == "google_apis"
    then "Google APIs"
    else if systemImage == "google_apis_playstore"
    then "Google Play"
    else if systemImage == "default"
    then "Default"
    else if systemImage == "aosp_atd"
    then "AOSP ATD"
    else if systemImage == "google_atd"
    then "Google ATD"
    else systemImage;

  mkCpuArch = arch:
    if arch == "arm64-v8a"
    then "arm64"
    else arch;

  mkCpuModel = arch:
    if arch == "x86_64"
    then "qemu64"
    else if arch == "x86"
    then "qemu32"
    else null;

  mkSkinName = name: avd:
    if avd.skin.name != null
    then avd.skin.name
    else name;

  mkSkinPath = name: avd:
    if avd.skin.path != null
    then avd.skin.path
    else "${skinRoot}/${mkSkinName name avd}";

  mkAvdIni = name: avd: ''
    avd.ini.encoding=UTF-8
    path=${avdRoot}/${name}.avd
    path.rel=avd/${name}.avd
    target=android-${avd.api}
  '';

  mkAvdConfig = name: avd: let
    cpuArch = mkCpuArch avd.arch;
    cpuModel = mkCpuModel avd.arch;
  in
    mkIni ({
        AvdId = name;
        "avd.ini.displayname" = avd.displayName;
        "avd.ini.encoding" = "UTF-8";

        "image.sysdir.1" = "system-images/android-${avd.api}/${avd.systemImage}/${avd.arch}/";
        "abi.type" = avd.arch;
        "hw.cpu.arch" = cpuArch;

        "tag.id" = avd.systemImage;
        "tag.display" = mkTagDisplay avd.systemImage;

        "hw.device.manufacturer" = avd.manufacturer;
        "hw.device.name" = avd.deviceName;

        "hw.lcd.width" = avd.width;
        "hw.lcd.height" = avd.height;
        "hw.lcd.density" = avd.density;

        "hw.ramSize" = avd.ramSize;
        "hw.cpu.ncore" = avd.cpuCores;

        "hw.keyboard" = avd.keyboard;
        "hw.mainKeys" = avd.mainKeys;

        "hw.gpu.enabled" = avd.gpu.enable;
        "hw.gpu.mode" = avd.gpu.mode;

        "disk.dataPartition.size" = avd.dataPartitionSize;
      }
      // optionalAttrs (cpuModel != null) {
        "hw.cpu.model" = cpuModel;
      }
      // optionalAttrs (avd.orientation != null) {
        "hw.initialOrientation" = avd.orientation;
      }
      // optionalAttrs avd.skin.enable {
        showDeviceFrame = avd.skin.showFrame;
        "skin.dynamic" = avd.skin.dynamic;
        "skin.name" = mkSkinName name avd;
        "skin.path" = mkSkinPath name avd;
      }
      // avd.extraConfig);

  mkAvdFiles =
    mapAttrs' (name: avd:
      nameValuePair ".android/avd/${name}.ini" {
        text = mkAvdIni name avd;
      })
    enabledAvds
    // mapAttrs' (name: avd:
      nameValuePair ".android/avd/${name}.avd/config.ini" {
        text = mkAvdConfig name avd;
      })
    enabledAvds;

  mkSkinFiles = mapAttrs' (name: avd:
    nameValuePair ".config/android/skins/${mkSkinName name avd}" {
      source = avd.skin.source;
      recursive = true;
    })
  (filterAttrs (_: avd: avd.enable && avd.skin.enable && avd.skin.source != null) cfg);

  mkLauncher = name: avd: let
    launcherName =
      if avd.launcher.name != null
      then avd.launcher.name
      else "android-emulator-${name}";

    extraArgs = concatStringsSep " " (map lib.escapeShellArg avd.launcher.extraArgs);
  in
    pkgs.writeShellScriptBin launcherName ''
      exec emulator @${lib.escapeShellArg name} ${extraArgs} "$@"
    '';

  launchers =
    mapAttrsToList mkLauncher
    (filterAttrs (_: avd: avd.enable && avd.launcher.enable) cfg);
in {
  options.android.avds = mkOption {
    default = {};
    description = "Declarative Android Virtual Devices. This generates AVD .ini files consumed by Android Emulator; Google does not document config.ini as a stable public API.";
    type = types.attrsOf (types.submodule ({name, ...}: {
      options = {
        enable = mkOption {
          type = types.bool;
          default = true;
          description = "Whether to generate this AVD.";
        };

        displayName = mkOption {
          type = types.str;
          default = name;
          description = "Human-readable AVD display name.";
        };

        api = mkOption {
          type = types.str;
          example = "36";
          description = "Android API level, used for target=android-<api> and image.sysdir.1.";
        };

        systemImage = mkOption {
          type = types.enum ["default" "google_apis" "google_apis_playstore" "aosp_atd" "google_atd"];
          default = "google_apis";
          description = "System image tag.";
        };

        arch = mkOption {
          type = types.enum ["x86" "x86_64" "arm64-v8a"];
          default = "x86_64";
          description = "System image architecture. Also used to derive hw.cpu.arch.";
        };

        manufacturer = mkOption {
          type = types.str;
          default = "Google";
          description = "Device manufacturer written to hw.device.manufacturer.";
        };

        deviceName = mkOption {
          type = types.str;
          default = name;
          description = "Device name written to hw.device.name.";
        };

        width = mkOption {
          type = types.int;
          description = "Display width in pixels, written to hw.lcd.width.";
        };

        height = mkOption {
          type = types.int;
          description = "Display height in pixels, written to hw.lcd.height.";
        };

        density = mkOption {
          type = types.int;
          description = "Display density in dpi, written to hw.lcd.density.";
        };

        ramSize = mkOption {
          type = types.int;
          default = 4096;
          description = "RAM size in MB, written to hw.ramSize.";
        };

        cpuCores = mkOption {
          type = types.int;
          default = 4;
          description = "Number of virtual CPU cores, written to hw.cpu.ncore.";
        };

        dataPartitionSize = mkOption {
          type = types.str;
          default = "8G";
          description = "AVD data partition size, written to disk.dataPartition.size.";
        };

        keyboard = mkOption {
          type = types.bool;
          default = true;
          description = "Hardware keyboard, written to hw.keyboard.";
        };

        mainKeys = mkOption {
          type = types.bool;
          default = false;
          description = "Hardware navigation keys, written to hw.mainKeys.";
        };

        orientation = mkOption {
          type = types.nullOr (types.enum ["Portrait" "Landscape"]);
          default = null;
          description = "Optional initial orientation, written to hw.initialOrientation.";
        };

        gpu = {
          enable = mkOption {
            type = types.bool;
            default = true;
            description = "GPU acceleration, written to hw.gpu.enabled.";
          };

          mode = mkOption {
            type = types.str;
            default = "auto";
            description = "GPU rendering mode, written to hw.gpu.mode.";
          };
        };

        skin = {
          enable = mkOption {
            type = types.bool;
            default = false;
            description = "Enable emulator skin entries.";
          };

          name = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Skin name written to skin.name.";
          };

          source = mkOption {
            type = types.nullOr types.path;
            default = null;
            description = "Local source directory containing extracted skin files.";
          };

          path = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Absolute runtime path written to skin.path. Defaults to ~/.config/android/skins/<skin-name>.";
          };

          dynamic = mkOption {
            type = types.bool;
            default = false;
            description = "Dynamic skin mode, written to skin.dynamic.";
          };

          showFrame = mkOption {
            type = types.bool;
            default = true;
            description = "Device frame visibility, written to showDeviceFrame.";
          };
        };

        launcher = {
          enable = mkOption {
            type = types.bool;
            default = true;
            description = "Generate an emulator launcher.";
          };

          name = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Launcher command name.";
          };

          extraArgs = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Extra arguments passed to emulator.";
          };
        };

        extraConfig = mkOption {
          type = types.attrsOf (types.oneOf [types.str types.int types.bool]);
          default = {};
          description = "Extra raw config.ini values. Use this for fields not modeled by this module.";
        };
      };
    }));
  };

  config = mkIf (enabledAvds != {}) {
    home.file = mkAvdFiles // mkSkinFiles;
    home.packages = launchers;

    home.activation.createAndroidAvdDirs = lib.hm.dag.entryAfter ["writeBoundary"] ''
      mkdir -p ${lib.escapeShellArg avdRoot}

      ${concatStringsSep "\n" (
        mapAttrsToList
        (name: _avd: "mkdir -p ${lib.escapeShellArg "${avdRoot}/${name}.avd"}")
        enabledAvds
      )}
    '';
  };
}

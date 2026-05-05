{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption types;

  cfg = config.services.go-librespot;

  yamlFormat = pkgs.formats.yaml {};
in {
  options.services.go-librespot = {
    enable = mkEnableOption "go-librespot Spotify Connect client";

    package = mkOption {
      type = types.package;
      default = pkgs.go-librespot;
      defaultText = lib.literalExpression "pkgs.go-librespot";
      description = "Package providing the go-librespot daemon.";
    };

    configDir = mkOption {
      type = types.path;
      default = "${config.xdg.configHome}/go-librespot";
      defaultText = lib.literalExpression ''"${config.xdg.configHome}/go-librespot"'';
      description = "Configuration directory used by go-librespot.";
    };

    settings = mkOption {
      type = types.submodule {
        freeformType = yamlFormat.type;

        options = {
          log_level = mkOption {
            type = types.enum [
              "panic"
              "fatal"
              "error"
              "warn"
              "warning"
              "info"
              "debug"
              "trace"
            ];
            default = "info";
            description = "Logging level for go-librespot.";
          };

          log_disable_timestamp = mkOption {
            type = types.bool;
            default = false;
            description = "Whether to disable timestamps in log output.";
          };

          device_id = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Spotify device ID in hex. Must be 40 characters when set.";
          };

          device_name = mkOption {
            type = types.str;
            default = "go-librespot";
            description = "Device name shown in Spotify clients.";
          };

          device_type = mkOption {
            type = types.enum [
              "computer"
              "tablet"
              "smartphone"
              "speaker"
              "tv"
              "avr"
              "stb"
              "audio_dongle"
              "game_console"
              "cast_video"
              "cast_audio"
              "automobile"
              "smartwatch"
              "chromebook"
              "car_thing"
              "observer"
              "home_thing"
            ];
            default = "computer";
            description = "Device type shown in Spotify clients.";
          };

          client_token = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Spotify client token.";
          };

          audio_backend = mkOption {
            type = types.enum [
              "alsa"
              "pulseaudio"
              "pipe"
            ];
            default = "alsa";
            description = "Audio backend used for playback.";
          };

          audio_backend_runtime_socket = mkOption {
            type = types.str;
            default = "";
            description = "Runtime socket for the audio backend. Mainly useful with PulseAudio.";
          };

          audio_device = mkOption {
            type = types.str;
            default = "default";
            description = "Audio device used for playback.";
          };

          mixer_device = mkOption {
            type = types.str;
            default = "";
            description = "Mixer device used for volume control. Empty disables mixer control.";
          };

          mixer_control_name = mkOption {
            type = types.str;
            default = "Master";
            description = "Mixer control name.";
          };

          audio_buffer_time = mkOption {
            type = types.ints.unsigned;
            default = 0;
            description = "Audio buffer time in microseconds. ALSA only. Zero means default.";
          };

          audio_period_count = mkOption {
            type = types.ints.unsigned;
            default = 0;
            description = "Number of audio periods to request. ALSA only. Zero means default.";
          };

          audio_output_pipe = mkOption {
            type = types.str;
            default = "";
            description = "Path to an existing FIFO for the pipe audio backend.";
          };

          audio_output_pipe_format = mkOption {
            type = types.enum [
              "s16le"
              "s32le"
              "f32le"
            ];
            default = "s16le";
            description = "Format of audio data written to the pipe backend.";
          };

          server = mkOption {
            type = types.submodule {
              options = {
                enabled = mkOption {
                  type = types.bool;
                  default = false;
                  description = "Whether to enable the API server.";
                };

                address = mkOption {
                  type = types.str;
                  default = "localhost";
                  description = "Address to bind the API server to.";
                };

                port = mkOption {
                  type = types.port;
                  default = 0;
                  description = "Port to bind the API server to. Zero means random/default.";
                };

                allow_origin = mkOption {
                  type = types.str;
                  default = "";
                  description = "Value for the Access-Control-Allow-Origin header.";
                };

                cert_file = mkOption {
                  type = types.str;
                  default = "";
                  description = "TLS certificate file.";
                };

                key_file = mkOption {
                  type = types.str;
                  default = "";
                  description = "TLS private key file.";
                };

                image_size = mkOption {
                  type = types.nullOr (types.enum [
                    "default"
                    "small"
                    "large"
                    "xlarge"
                  ]);
                  default = null;
                  description = "Preferred album cover image size served by the API.";
                };
              };
            };
            default = {};
            description = "API server configuration.";
          };

          zeroconf_enabled = mkOption {
            type = types.bool;
            default = false;
            description = "Whether Zeroconf discovery should always be enabled.";
          };

          zeroconf_port = mkOption {
            type = types.port;
            default = 0;
            description = "Port used for Zeroconf. Zero means random.";
          };

          zeroconf_backend = mkOption {
            type = types.enum [
              "builtin"
              "avahi"
            ];
            default = "builtin";
            description = "mDNS backend used for Zeroconf registration.";
          };

          zeroconf_interfaces_to_advertise = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Network interfaces advertised through Zeroconf. Empty means all present interfaces.";
          };

          credentials = mkOption {
            type = types.submodule {
              options = {
                type = mkOption {
                  type = types.enum [
                    "interactive"
                    "spotify_token"
                    "zeroconf"
                  ];
                  default = "zeroconf";
                  description = "Authentication method.";
                };

                interactive = mkOption {
                  type = types.submodule {
                    options.callback_port = mkOption {
                      type = types.port;
                      default = 0;
                      description = "Callback port for interactive authentication.";
                    };
                  };
                  default = {};
                  description = "Interactive browser authentication settings.";
                };

                spotify_token = mkOption {
                  type = types.submodule {
                    options = {
                      username = mkOption {
                        type = types.str;
                        default = "";
                        description = "Spotify username.";
                      };

                      access_token = mkOption {
                        type = types.str;
                        default = "";
                        description = "Spotify access token.";
                      };
                    };
                  };
                  default = {};
                  description = "Spotify token authentication settings.";
                };

                zeroconf = mkOption {
                  type = types.submodule {
                    options.persist_credentials = mkOption {
                      type = types.bool;
                      default = false;
                      description = "Whether credentials from Zeroconf sessions should be persisted.";
                    };
                  };
                  default = {};
                  description = "Zeroconf authentication settings.";
                };
              };
            };
            default = {};
            description = "Authentication configuration.";
          };

          bitrate = mkOption {
            type = types.enum [
              96
              160
              320
            ];
            default = 160;
            description = "Preferred playback bitrate.";
          };

          volume_steps = mkOption {
            type = types.ints.positive;
            default = 100;
            description = "Number of volume steps.";
          };

          initial_volume = mkOption {
            type = types.ints.between 0 100;
            default = 100;
            description = "Initial volume in steps.";
          };

          ignore_last_volume = mkOption {
            type = types.bool;
            default = false;
            description = "Whether to ignore the last saved volume.";
          };

          normalisation_disabled = mkOption {
            type = types.bool;
            default = false;
            description = "Whether track/album normalisation should be disabled.";
          };

          normalisation_use_album_gain = mkOption {
            type = types.bool;
            default = false;
            description = "Whether album gain should be used instead of track gain.";
          };

          normalisation_pregain = mkOption {
            type = types.number;
            default = 0;
            description = "Pregain applied to track/album normalisation factors.";
          };

          external_volume = mkOption {
            type = types.bool;
            default = false;
            description = "Whether volume is controlled externally.";
          };

          disable_autoplay = mkOption {
            type = types.bool;
            default = false;
            description = "Whether autoplay of more songs should be disabled.";
          };

          mpris_enabled = mkOption {
            type = types.bool;
            default = false;
            description = "Whether to enable MPRIS communication through D-Bus.";
          };

          flac_enabled = mkOption {
            type = types.bool;
            default = false;
            description = "Whether FLAC files should be preferred when available.";
          };
        };
      };

      default = {};
      description = "go-librespot configuration written to config.yml.";
    };

    service = {
      enable = mkOption {
        type = types.bool;
        default = cfg.enable;
        defaultText = lib.literalExpression "config.services.go-librespot.enable";
        description = "Whether to enable the go-librespot user service.";
      };

      extraArgs = mkOption {
        type = types.listOf types.str;
        default = [];
        description = "Extra arguments passed to go-librespot.";
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile."go-librespot/config.yml".source =
      yamlFormat.generate "go-librespot-config.yml"
      (lib.filterAttrsRecursive (_: value: value != null) cfg.settings);

    systemd.user.services.go-librespot = mkIf cfg.service.enable {
      Unit = {
        Description = "go-librespot Spotify Connect client";
        After = [
          "pulseaudio.service"
          "pipewire-pulse.service"
        ];
        Wants = [
          "pulseaudio.service"
          "pipewire-pulse.service"
        ];
      };

      Service = {
        ExecStart = lib.escapeShellArgs ([
            "${cfg.package}/bin/go-librespot"
            "--config_dir"
            cfg.configDir
          ]
          ++ cfg.service.extraArgs);

        Restart = "on-failure";
        RestartSec = 3;
      };

      Install = {
        WantedBy = ["default.target"];
      };
    };
  };
}

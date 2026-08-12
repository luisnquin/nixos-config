{
  config,
  lib,
  pkgs,
  ...
}: let
  asrModelId = "voice-bilingual-whisper-small-q8";
  asrModel = pkgs.fetchurl {
    url = "https://huggingface.co/handy-computer/whisper-small-gguf/resolve/main/whisper-small-Q8_0.gguf";
    hash = "sha256-m5yIEbvMgqd2bw+wklYUvaywkjssxjDa6sFxCLZVuGA=";
  };

  englishVoice = {
    model = pkgs.fetchurl {
      url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx";
      hash = "sha256-Xv4J5pkCGHgnr2RuGm6dJp3udp+Yd9F7FrG0buqvAZ8=";
    };
    config = pkgs.fetchurl {
      url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json";
      hash = "sha256-7+GcQXvtBV8taZCCSMa6ZQ+hNbyGiw5quz2hgdq2kKA=";
    };
  };

  spanishVoice = {
    model = pkgs.fetchurl {
      url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_AR/daniela/high/es_AR-daniela-high.onnx";
      hash = "sha256-fOsfwNqzSUGMW1SmOa6e5ZUhLXyepCIiDYQZFj1cyYU=";
    };
    config = pkgs.fetchurl {
      url = "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_AR/daniela/high/es_AR-daniela-high.onnx.json";
      hash = "sha256-rtv2lkfh11TGLs+OA2bKXxavPnaOPGtTKa9utr3jhSs=";
    };
  };

  englishVoiceDir = pkgs.linkFarm "piper-voice-en_US-lessac-medium" [
    {
      name = "en_US-lessac-medium.onnx";
      path = englishVoice.model;
    }
    {
      name = "en_US-lessac-medium.onnx.json";
      path = englishVoice.config;
    }
  ];

  spanishVoiceDir = pkgs.linkFarm "piper-voice-es_AR-daniela-high" [
    {
      name = "es_AR-daniela-high.onnx";
      path = spanishVoice.model;
    }
    {
      name = "es_AR-daniela-high.onnx.json";
      path = spanishVoice.config;
    }
  ];

  voiceReplyPlayer = pkgs.writeShellApplication {
    name = "voice-reply-player";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.pipewire
      pkgs.piper-tts
    ];
    text = ''
      reply_language="$1"
      shift

      case "$reply_language" in
        en)
          reply_model=${lib.escapeShellArg "${englishVoiceDir}/en_US-lessac-medium.onnx"}
          ;;
        es)
          reply_model=${lib.escapeShellArg "${spanishVoiceDir}/es_AR-daniela-high.onnx"}
          ;;
        *)
          printf 'voice-reply: unsupported language: %s\n' "$reply_language" >&2
          exit 2
          ;;
      esac

      reply_dir="$(mktemp -d)"
      trap 'rm -rf -- "$reply_dir"' EXIT
      reply_wav="$reply_dir/reply.wav"

      piper \
        --model "$reply_model" \
        --output-file "$reply_wav" \
        -- "$*"
      pw-play "$reply_wav"
    '';
  };

  voiceReply = pkgs.writeShellApplication {
    name = "voice-reply";
    runtimeInputs = [pkgs.systemd];
    text = ''
      if [ "''${1:-}" = "--cancel" ]; then
        systemctl --user stop voice-reply.service >/dev/null 2>&1 || true
        exit 0
      fi

      if [ "''${1:-}" != "--lang" ] || [ "$#" -lt 3 ]; then
        printf 'usage: voice-reply --lang <en|es> <text>\n' >&2
        exit 2
      fi

      reply_language="$2"
      shift 2

      systemctl --user stop voice-reply.service >/dev/null 2>&1 || true
      systemctl --user reset-failed voice-reply.service >/dev/null 2>&1 || true
      systemd-run \
        --user \
        --quiet \
        --collect \
        --unit=voice-reply.service \
        --service-type=exec \
        -- ${lib.getExe voiceReplyPlayer} "$reply_language" "$*"
    '';
  };

  voiceGateway = pkgs.writeShellApplication {
    name = "voice-gateway";
    runtimeInputs = [
      config.programs.codex.package
      pkgs.herdr
      pkgs.rtk
      pkgs.voice-gateway
    ];
    text = ''
      export VOICE_CODEX_BIN="''${VOICE_CODEX_BIN:-${lib.getExe config.programs.codex.package}}"
      export VOICE_CODEX_MODEL="''${VOICE_CODEX_MODEL:-gpt-5.6-terra}"
      export VOICE_CODEX_EFFORT="''${VOICE_CODEX_EFFORT:-low}"
      export VOICE_THREAD_MAX_TURNS="''${VOICE_THREAD_MAX_TURNS:-12}"
      export VOICE_HERDR_BIN="''${VOICE_HERDR_BIN:-${lib.getExe pkgs.herdr}}"
      export VOICE_REPLY_BIN="''${VOICE_REPLY_BIN:-${lib.getExe voiceReply}}"
      export VOICE_SKILL_PATH="''${VOICE_SKILL_PATH:-${config.home.homeDirectory}/.agents/skills/voice-orchestrator/SKILL.md}"
      export VOICE_HERDR_SESSION="''${VOICE_HERDR_SESSION:-hub}"
      export HERDR_ENV="''${HERDR_ENV:-1}"
      exec ${lib.getExe pkgs.voice-gateway} "$@"
    '';
  };

  voiceAgentInput = pkgs.writeShellApplication {
    name = "voice-agent-input";
    runtimeInputs = [pkgs.systemd];
    text = ''
      if [ "$#" -ne 1 ]; then
        printf 'usage: voice-agent-input <transcript>\n' >&2
        exit 2
      fi

      systemctl --user start voice-gateway.service
      exec ${lib.getExe voiceGateway} submit "$1"
    '';
  };

  voiceAgentToggle = pkgs.writeShellApplication {
    name = "voice-agent-toggle";
    text = ''
      ${lib.getExe voiceReply} --cancel
      exec ${lib.getExe pkgs.handy} --toggle-transcription
    '';
  };

  voiceAgentCancel = pkgs.writeShellApplication {
    name = "voice-agent-cancel";
    text = ''
      ${lib.getExe voiceReply} --cancel
      ${lib.getExe voiceGateway} cancel >/dev/null 2>&1 || true
      exec ${lib.getExe pkgs.handy} --cancel
    '';
  };

  handyVoiceRouting = pkgs.writeShellApplication {
    name = "handy-voice-routing";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.jq
    ];
    text = ''
      settings_path="''${XDG_DATA_HOME:-$HOME/.local/share}/com.pais.handy/settings_store.json"
      settings_tmp="$(mktemp "''${settings_path}.hm-tmp.XXXXXX")"
      trap 'rm -f -- "$settings_tmp"' EXIT

      mkdir -p "$(dirname "$settings_path")"

      if [ -f "$settings_path" ]; then
        jq \
          --arg script ${lib.escapeShellArg (lib.getExe voiceAgentInput)} \
          --arg model ${lib.escapeShellArg asrModelId} \
          '.settings.paste_method = "external_script"
           | .settings.external_script_path = $script
           | .settings.selected_language = "auto"
           | .settings.selected_model = $model
           | .settings.onboarding_completed = true
           | .settings.autostart_enabled = false
           | .settings.start_hidden = true' \
          "$settings_path" > "$settings_tmp"
      else
        jq \
          --null-input \
          --arg script ${lib.escapeShellArg (lib.getExe voiceAgentInput)} \
          --arg model ${lib.escapeShellArg asrModelId} \
          '{settings: {
            paste_method: "external_script",
            external_script_path: $script,
            selected_language: "auto",
            selected_model: $model,
            onboarding_completed: true,
            autostart_enabled: false,
            start_hidden: true
          }}' > "$settings_tmp"
      fi

      mv -f "$settings_tmp" "$settings_path"
    '';
  };
in {
  home = {
    packages = [
      pkgs.handy
      voiceAgentCancel
      voiceAgentInput
      voiceAgentToggle
      voiceGateway
      voiceReply
    ];

    file.".local/share/com.pais.handy/models/${asrModelId}.gguf".source = asrModel;

    # Handy owns this mutable Tauri store. Preserve its device and UI choices.
    activation.handyVoiceRouting = lib.hm.dag.entryAfter ["writeBoundary"] ''
      run ${lib.getExe handyVoiceRouting}
    '';
  };

  systemd.user.services.voice-gateway = {
    Unit = {
      Description = "Persistent Codex voice orchestrator";
      After = ["network-online.target"];
      Wants = ["network-online.target"];
      X-Restart-Triggers = [voiceGateway];
    };

    Service = {
      Type = "simple";
      ExecStart = "${lib.getExe voiceGateway} serve";
      Restart = "on-failure";
      RestartSec = 1;
    };

    Install.WantedBy = ["default.target"];
  };

  systemd.user.services.handy = {
    Unit = {
      Description = "Handy speech-to-text";
      After = [
        "graphical-session.target"
        "voice-gateway.service"
      ];
      PartOf = ["graphical-session.target"];
      Wants = ["voice-gateway.service"];
      X-Restart-Triggers = [
        asrModel
        handyVoiceRouting
        voiceAgentInput
        voiceGateway
      ];
    };

    Service = {
      Type = "simple";
      ExecStartPre = lib.getExe handyVoiceRouting;
      ExecStart = "${lib.getExe pkgs.handy} --start-hidden";
      Restart = "on-failure";
    };

    Install.WantedBy = ["graphical-session.target"];
  };
}

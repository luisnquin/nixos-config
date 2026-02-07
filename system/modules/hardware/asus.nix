{
  services.power-profiles-daemon.enable = true;

  services.asusd = {
    enable = true;
    enableUserService = true;

    asusdConfig.text = ''
      "bat_charge_limit": 80
    '';

    profileConfig.text = ''
      (
        active_profile: Balanced,
        profiles: {
          Balanced: (
            platform_profile: Balanced,
            fan_curve: (cpu: [], gpu: []),
          ),
          Performance: (
            platform_profile: Performance,
            fan_curve: (cpu: [], gpu: []),
          ),
          Quiet: (
            platform_profile: Quiet,
            fan_curve: (cpu: [], gpu: []),
          ),
        },
      )
    '';

    auraConfigs.tuf.text = ''
      (
        brightness: Med,
        current_mode: Pulse,
        builtins: {
          Static: (
            mode: Static,
            zone: r#None,
            colour1: (r: 200, g: 80, b: 30),
            colour2: (r: 0, g: 0, b: 0),
            speed: Med,
            direction: Right,
          ),
          Breathe: (
            mode: Breathe,
            zone: r#None,
            colour1: (r: 200, g: 80, b: 30),
            colour2: (r: 30, g: 200, b: 80),
            speed: Low,
            direction: Right,
          ),
          Pulse: (
            mode: Pulse,
            zone: r#None,
            colour1: (r: 200, g: 80, b: 30),
            colour2: (r: 0, g: 0, b: 0),
            speed: Low,                        # Low = transición más lenta y suave
            direction: Right,
          ),
        },
        multizone_on: false,
        enabled: (
          states: [
            (
              zone: Keyboard,
              boot: true,
              awake: true,
              sleep: false,      # Se apaga al suspender
              shutdown: false,
            ),
          ],
        ),
      )
    '';
  };
}

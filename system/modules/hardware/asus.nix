{
  services.power-profiles-daemon.enable = true;

  services.asusd = {
    enable = true;
    enableUserService = false;

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

    fanCurvesConfig.text = ''
      (
        Balanced: (
          cpu: [
            (temp: 30, pwm: 0),
            (temp: 40, pwm: 5),
            (temp: 50, pwm: 15),
            (temp: 60, pwm: 30),
            (temp: 70, pwm: 50),
            (temp: 80, pwm: 70),
            (temp: 90, pwm: 85),
            (temp: 100, pwm: 100),
          ],
          gpu: [
            (temp: 30, pwm: 0),
            (temp: 40, pwm: 5),
            (temp: 50, pwm: 15),
            (temp: 60, pwm: 30),
            (temp: 70, pwm: 50),
            (temp: 80, pwm: 70),
            (temp: 90, pwm: 85),
            (temp: 100, pwm: 100),
          ],
        ),
        Performance: (
          cpu: [
            (temp: 30, pwm: 15),
            (temp: 40, pwm: 25),
            (temp: 50, pwm: 40),
            (temp: 60, pwm: 55),
            (temp: 70, pwm: 70),
            (temp: 80, pwm: 85),
            (temp: 90, pwm: 95),
            (temp: 100, pwm: 100),
          ],
          gpu: [
            (temp: 30, pwm: 15),
            (temp: 40, pwm: 25),
            (temp: 50, pwm: 40),
            (temp: 60, pwm: 55),
            (temp: 70, pwm: 70),
            (temp: 80, pwm: 85),
            (temp: 90, pwm: 95),
            (temp: 100, pwm: 100),
          ],
        ),
        Quiet: (
          cpu: [
            (temp: 30, pwm: 0),
            (temp: 40, pwm: 0),
            (temp: 50, pwm: 8),
            (temp: 60, pwm: 18),
            (temp: 70, pwm: 35),
            (temp: 80, pwm: 55),
            (temp: 90, pwm: 70),
            (temp: 100, pwm: 80),
          ],
          gpu: [
            (temp: 30, pwm: 0),
            (temp: 40, pwm: 0),
            (temp: 50, pwm: 8),
            (temp: 60, pwm: 18),
            (temp: 70, pwm: 35),
            (temp: 80, pwm: 55),
            (temp: 90, pwm: 70),
            (temp: 100, pwm: 80),
          ],
        ),
      )
    '';
  };
}

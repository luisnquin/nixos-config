use std::time::Duration;

const INJECTED: [i64; 2] = [0, -1];

pub fn last_human(dump: &str) -> Option<Duration> {
    recent(dump)
        .filter_map(event)
        .filter(|(device, _)| !INJECTED.contains(device))
        .map(|(_, age)| age)
        .min()
}

fn recent(dump: &str) -> impl Iterator<Item = &str> {
    let mut lines = dump.lines();
    let head = lines.by_ref().find(|line| line.trim_start().starts_with("RecentQueue:"));
    let depth = head.map_or(usize::MAX, indent);

    lines
        .take_while(move |line| head.is_some() && indent(line) > depth)
        .map(str::trim)
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn event(line: &str) -> Option<(i64, Duration)> {
    let device = field(line, "deviceId=")?.parse().ok()?;
    let age = age(field(line, "age=")?)?;

    Some((device, age))
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find([',', ')', ' ']).unwrap_or(rest.len());

    Some(&rest[..end])
}

fn age(text: &str) -> Option<Duration> {
    let split = text.find(|c: char| c.is_ascii_alphabetic())?;
    let (number, unit) = text.split_at(split);
    let number: f64 = number.parse().ok()?;
    let seconds = match unit {
        "ms" => number / 1000.0,
        "s" => number,
        "m" | "min" => number * 60.0,
        _ => return None,
    };

    Duration::try_from_secs_f64(seconds).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMP: &str = "INPUT MANAGER (dumpsys input)

Input Dispatcher State:
  DispatchEnabled: true
  RecentQueue: length=4
    MotionEvent(deviceId=4, eventTime=8150000000, source=TOUCHSCREEN, displayId=0, action=DOWN, pointers=[0: (540.0, 1200.0)]), policyFlags=0x62000000, age=91234.5ms
    MotionEvent(deviceId=4, eventTime=8160000000, source=TOUCHSCREEN, displayId=0, action=UP, pointers=[0: (540.0, 1200.0)]), policyFlags=0x62000000, age=91120.0ms
    MotionEvent(deviceId=-1, eventTime=8170000000, source=TOUCHSCREEN, displayId=0, action=DOWN), policyFlags=0x40000000, age=2003.1ms
    KeyEvent(deviceId=0, eventTime=8180000000, source=KEYBOARD, displayId=-1, action=UP, keyCode=KEYCODE_BACK(4)), policyFlags=0x40000000, age=1500.0ms
  PendingEvent: <none>
  InboundQueue: <empty>
    MotionEvent(deviceId=7, eventTime=8190000000, source=TOUCHSCREEN), age=10.0ms
";

    #[test]
    fn the_youngest_event_from_a_real_device_is_the_last_touch() {
        assert_eq!(last_human(DUMP), Some(Duration::from_millis(91120)));
    }

    #[test]
    fn injected_events_and_those_outside_the_recent_queue_are_ignored() {
        for injected in ["deviceId=-1", "deviceId=0"] {
            assert_eq!(last_human(&DUMP.replace("deviceId=4", injected)), None);
        }
    }

    #[test]
    fn an_empty_recent_queue_is_no_touch() {
        assert_eq!(last_human("Input Dispatcher State:\n  RecentQueue: <empty>\n  PendingEvent: <none>\n"), None);
    }

    #[test]
    fn a_dump_without_a_recent_queue_is_no_touch() {
        assert_eq!(last_human("Can't find service: input\n"), None);
        assert_eq!(last_human(""), None);
    }

    #[test]
    fn a_line_without_an_age_is_skipped() {
        let dump = "  RecentQueue: length=2\n    MotionEvent(deviceId=4, action=DOWN)\n    MotionEvent(deviceId=5, action=UP), age=3s\n";

        assert_eq!(last_human(dump), Some(Duration::from_secs(3)));
    }

    #[test]
    fn ages_carry_their_unit() {
        assert_eq!(age("250ms"), Some(Duration::from_millis(250)));
        assert_eq!(age("1.5s"), Some(Duration::from_millis(1500)));
        assert_eq!(age("2m"), Some(Duration::from_secs(120)));
        assert_eq!(age("12"), None);
        assert_eq!(age("-3ms"), None);
        assert_eq!(age("3h"), None);
    }
}

use std::path::{Path, PathBuf};

use super::Check;
use crate::project::{Manifest, Spec};

const SCALES: &str = "phone:scales";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scales {
    window: f64,
    transition: f64,
    animator: f64,
}

impl Scales {
    pub fn parse(said: &str) -> Option<Self> {
        let line = said.lines().find_map(|l| l.trim().strip_prefix(SCALES))?;
        let read = |v: &str| match v {
            "null" => Some(1.0),
            v => v.parse::<f64>().ok(),
        };

        let [window, transition, animator] = line
            .split_whitespace()
            .map(read)
            .collect::<Option<Vec<f64>>>()?
            .try_into()
            .ok()?;

        Some(Scales {
            window,
            transition,
            animator,
        })
    }
}

#[derive(Debug)]
pub struct Pin {
    file: PathBuf,
    section: &'static str,
    value: String,
}

impl Pin {
    pub fn of(device: &crate::model::Device) -> Option<Self> {
        let project = crate::project::Project::here().ok().flatten()?;

        let section = device.platform.os();

        Some(Pin {
            file: project.root.join(crate::project::FILE),
            section,
            value: transition(&project.manifest.spec(section))?,
        })
    }
}

fn transition(spec: &Spec) -> Option<String> {
    spec.settings
        .global
        .get("transition_animation_scale")
        .map(ToString::to_string)
}

fn nonzero(value: &str) -> bool {
    value.parse::<f64>().is_ok_and(|v| v != 0.0)
}

pub fn not_idle(scales: Option<Scales>, pin: Option<&Pin>) -> String {
    let head = "the screen never went idle, so uiautomator would not read it: something on it \
                animates without end.";

    let Some(Scales {
        window,
        transition,
        animator,
    }) = scales
    else {
        return format!(
            "{head} Turn on Remove animations, or set the animator, transition and window \
             animation scales to 0 in Developer options"
        );
    };

    if transition != 0.0 {
        let fix = match pin {
            Some(p) if !nonzero(&p.value) => format!(
                "{} already pins it to 0 under [{}] settings.global, so the device \
                 drifted; `phone up` puts it back and restarts the app",
                p.file.display(),
                p.section
            ),
            Some(p) => format!(
                "{} pins it to {} under [{}] settings.global; set it to 0 there and \
                 run `phone up`, which restarts the app",
                p.file.display(),
                p.value,
                p.section
            ),
            None => "Turn on Remove animations, or set the animator, transition and window \
                     animation scales to 0 in Developer options, and restart the app"
                .to_string(),
        };

        return format!(
            "{head} Reduced motion is off on the device (transition_animation_scale is \
             {transition}), so Reanimated's useReducedMotion() is false. {fix}"
        );
    }

    let rest = match window == 0.0 && animator == 0.0 {
        true => String::new(),
        false => format!(
            " The window and animator scales are {window} and {animator}; set them to 0 too."
        ),
    };

    format!(
        "{head} Reduced motion is on, so either the app started before it was and still \
         reads it as off (`phone app stop`, then launch it again), or the loop ignores it: a \
         Reanimated animation set to ReduceMotion.Never, or one outside Reanimated (Lottie, a \
         native view).{rest} `phone shot` shows what is on screen"
    )
}

pub fn pins(manifest: &Manifest, file: &Path) -> Vec<Check> {
    manifest
        .android
        .iter()
        .filter_map(|spec| {
            let value = transition(spec).filter(|v| nonzero(v))?;

            Some(Check::warn(
                "android",
                format!(
                    "{} pins transition_animation_scale to {value}, so Reanimated's \
                     useReducedMotion() stays false there and an endless animation blocks \
                     snapshot, tap, wait and fill; set it to 0 unless the app needs real \
                     animation timing",
                    file.display()
                ),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::Verdict;
    use crate::project::Project;

    fn pin(value: &str) -> Pin {
        Pin {
            file: PathBuf::from("/src/app/phone.toml"),
            section: "android",
            value: value.to_string(),
        }
    }

    const ON: Scales = Scales {
        window: 1.0,
        transition: 1.0,
        animator: 1.0,
    };

    const OFF: Scales = Scales {
        window: 0.0,
        transition: 0.0,
        animator: 0.0,
    };

    #[test]
    fn an_unread_scale_keeps_the_plain_advice() {
        let msg = not_idle(None, Some(&pin("1")));

        assert!(msg.contains("Remove animations"), "{msg}");
        assert!(!msg.contains("phone.toml"), "{msg}");
    }

    #[test]
    fn a_nonzero_transition_scale_is_reduced_motion_off() {
        let msg = not_idle(Some(ON), None);

        assert!(msg.contains("Reduced motion is off"), "{msg}");
        assert!(msg.contains("useReducedMotion() is false"), "{msg}");
        assert!(msg.contains("Developer options"), "{msg}");
    }

    #[test]
    fn a_manifest_that_pins_the_scale_is_named() {
        let msg = not_idle(Some(ON), Some(&pin("1")));

        assert!(msg.contains("/src/app/phone.toml pins it to 1"), "{msg}");
        assert!(msg.contains("[android] settings.global"), "{msg}");

        let msg = not_idle(Some(ON), Some(&pin("0")));
        assert!(msg.contains("already pins it to 0"), "{msg}");
        assert!(msg.contains("`phone up` puts it back"), "{msg}");
        assert!(!msg.contains("Developer options"), "{msg}");
    }

    #[test]
    fn with_every_scale_at_zero_the_loop_ignores_reduced_motion() {
        let msg = not_idle(Some(OFF), Some(&pin("1")));

        assert!(msg.contains("Reduced motion is on"), "{msg}");
        assert!(msg.contains("ReduceMotion.Never"), "{msg}");
        assert!(msg.contains("started before"), "{msg}");
        assert!(msg.contains("phone shot"), "{msg}");
        assert!(!msg.contains("set them to 0"), "{msg}");

        let msg = not_idle(
            Some(Scales {
                animator: 1.0,
                ..OFF
            }),
            None,
        );
        assert!(msg.contains("Reduced motion is on"), "{msg}");
        assert!(msg.contains("animator scales are 0 and 1"), "{msg}");
    }

    fn checked(toml: &str) -> Vec<Check> {
        let manifest = Project::parse(toml).unwrap();

        pins(&manifest, Path::new("/src/app/phone.toml"))
    }

    #[test]
    fn a_profile_pinned_to_a_nonzero_scale_is_a_warning() {
        let found = checked(
            r#"
            [android.settings.global]
            transition_animation_scale = "0.5"
            window_animation_scale = 0
            "#,
        );

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].verdict, Verdict::Warn);
        assert_eq!(found[0].name, "android");
        assert!(found[0]
            .detail
            .contains("/src/app/phone.toml pins transition_animation_scale to 0.5"));
        assert!(found[0].detail.contains("useReducedMotion() stays false"));
        assert!(found[0].detail.contains("snapshot, tap, wait and fill"));
    }

    #[test]
    fn a_scale_at_zero_or_left_alone_is_not_reported() {
        let found = checked(
            r#"
            [android.settings.global]
            transition_animation_scale = "0.0"

            [android.settings.system]
            transition_animation_scale = 1

            [ios]
            state = "ready"
            "#,
        );

        assert!(
            found.is_empty(),
            "{:?}",
            found.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }
}

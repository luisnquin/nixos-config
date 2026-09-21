use ttycanvas::audio::{Cue, Options, Sound};

use crate::app::{AppState, Field, Phase};
use crate::config::SoundConfig;

pub const TICK_MS: u64 = 120;

pub fn open(cfg: &SoundConfig) -> Option<Sound> {
    if !cfg.enable {
        return None;
    }
    let options = Options {
        device: cfg.device.clone(),
        volume: cfg.volume,
        tick_ms: TICK_MS,
    };
    Sound::open(options, crate::ascii_animation::carousel())
}

pub struct Snapshot {
    phase: Phase,
    typed: usize,
    focus: Field,
    logs_open: bool,
}

pub fn snapshot(app: &AppState) -> Snapshot {
    Snapshot {
        phase: app.phase.clone(),
        typed: app.user.len() + app.password.len(),
        focus: app.focus,
        logs_open: app.logs_open,
    }
}

pub fn cues(before: &Snapshot, app: &AppState) -> Vec<Cue> {
    let mut out = Vec::new();
    if app.focus != before.focus || app.logs_open != before.logs_open {
        out.push(Cue::Nav);
    }
    if app.phase == before.phase {
        let typed = app.user.len() + app.password.len();
        if typed > before.typed {
            out.push(Cue::Key);
        } else if typed < before.typed {
            out.push(Cue::Erase);
        }
        return out;
    }
    match &app.phase {
        Phase::Creating => {
            out.push(Cue::Submit);
            out.push(Cue::Scan(true));
        }
        Phase::Authenticating => {
            if matches!(before.phase, Phase::Prompt { .. }) {
                out.push(Cue::Submit);
            }
            out.push(Cue::Scan(true));
        }
        Phase::Starting => out.push(Cue::Scan(true)),
        Phase::Prompt { .. } => out.push(Cue::Scan(false)),
        Phase::Failed(_) => {
            out.push(Cue::Scan(false));
            out.push(Cue::Fail);
        }
        Phase::Idle => {
            if !matches!(before.phase, Phase::Failed(_)) {
                out.push(Cue::Cancel);
            } else {
                out.push(Cue::Key);
            }
        }
        Phase::Done => out.push(Cue::Grant),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Action;
    use crate::greetd::{AuthMessageType, ErrorType, Response};

    fn app() -> AppState {
        AppState::new("b".into(), "user".into(), vec!["s".into()], true)
    }

    fn step(app: &mut AppState, action: Action) -> Vec<Cue> {
        let before = snapshot(app);
        app.update(action);
        cues(&before, app)
    }

    #[test]
    fn typing_and_erasing() {
        let mut a = app();
        assert_eq!(step(&mut a, Action::Char('x')), vec![Cue::Key]);
        assert_eq!(step(&mut a, Action::Backspace), vec![Cue::Erase]);
        assert_eq!(step(&mut a, Action::Backspace), vec![]);
        assert_eq!(step(&mut a, Action::FocusToggle), vec![Cue::Nav]);
    }

    #[test]
    fn the_auth_chain_scans_then_grants() {
        let mut a = app();
        a.password = "pw".into();
        assert_eq!(
            step(&mut a, Action::Submit),
            vec![Cue::Submit, Cue::Scan(true)]
        );
        let prompt = Action::Greetd(Response::AuthMessage {
            auth_message_type: AuthMessageType::Secret,
            auth_message: "Password: ".into(),
        });
        assert_eq!(step(&mut a, prompt), vec![Cue::Scan(true)]);
        assert_eq!(
            step(&mut a, Action::Greetd(Response::Success)),
            vec![Cue::Scan(true)]
        );
        assert_eq!(
            step(&mut a, Action::Greetd(Response::Success)),
            vec![Cue::Grant]
        );
    }

    #[test]
    fn a_rejection_fails_and_typing_after_it_is_a_key() {
        let mut a = app();
        a.password = "pw".into();
        step(&mut a, Action::Submit);
        let err = Action::Greetd(Response::Error {
            error_type: ErrorType::AuthError,
            description: String::new(),
        });
        assert_eq!(step(&mut a, err), vec![Cue::Scan(false), Cue::Fail]);
        assert_eq!(step(&mut a, Action::Char('q')), vec![Cue::Key]);
    }

    #[test]
    fn escape_mid_chain_cancels() {
        let mut a = app();
        step(&mut a, Action::Submit);
        assert_eq!(step(&mut a, Action::Cancel), vec![Cue::Cancel]);
    }
}

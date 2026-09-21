use crate::effects::Effects;
use crate::machine::{Action, Event, Machine, Phase, State};

const MAX_FAILURE_ROUNDS: usize = 4;

pub struct Runner<E: Effects> {
    machine: Machine,
    effects: E,
    failure: Option<String>,
    terminated: bool,
    health: Option<String>,
}

impl<E: Effects> Runner<E> {
    pub fn new(machine: Machine, effects: E) -> Self {
        Self {
            machine,
            effects,
            failure: None,
            terminated: false,
            health: None,
        }
    }

    pub fn state(&self) -> State {
        self.machine.state()
    }

    pub fn phase(&self) -> Phase {
        self.machine.phase()
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn effects(&self) -> &E {
        &self.effects
    }

    pub fn last_dispatch_failed(&self) -> bool {
        self.failure.is_some()
    }

    pub fn last_dispatch_terminated(&self) -> bool {
        self.terminated
    }

    pub fn health(&self) -> Option<&str> {
        self.health.as_deref()
    }

    pub fn dispatch(&mut self, event: Event) -> State {
        let before = self.machine.state();
        self.failure = None;
        self.terminated = false;
        if matches!(event, Event::Arm | Event::Disarm) {
            self.health = None;
        }

        let mut pending = self.machine.handle(event);
        for round in 0..=MAX_FAILURE_ROUNDS {
            let mut failed = None;
            for action in pending.drain(..) {
                if let Err(err) = self.apply(action) {
                    failed = Some((action, err));
                    break;
                }
            }

            let Some((action, err)) = failed else { break };
            let detail = format!("{action:?} failed: {err}");
            crate::log(&detail);
            self.failure = Some(detail.clone());
            self.health = Some(detail);

            if round == MAX_FAILURE_ROUNDS {
                crate::log("too many cascading effect failures; abandoning this batch");
                break;
            }
            pending = self.machine.handle(Event::ActionFailed {
                action,
                now: self.effects.now(),
            });
        }

        let after = self.machine.state();
        if before != after {
            crate::log(&format!("{before:?} -> {after:?}"));
        }
        after
    }

    fn apply(&mut self, action: Action) -> std::io::Result<()> {
        match action {
            Action::TerminateUserSession { grace } => {
                self.effects.terminate_user_session(grace)?;
                self.terminated = true;
                Ok(())
            }
            Action::ArmWake { after } => self.effects.arm_wake(after),
            Action::CancelWake => self.effects.cancel_wake(),
            Action::Suspend => self.effects.suspend(),
        }
    }
}

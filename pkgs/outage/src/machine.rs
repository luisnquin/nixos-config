use std::time::Duration;

pub type Millis = u64;

pub const MAX_TEARDOWN_ATTEMPTS: u32 = 3;
pub const MAX_SLEEP_FAILURES: u32 = 3;

fn ms(d: Duration) -> Millis {
    d.as_millis() as Millis
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    pub offline_grace: Duration,
    pub idle_grace: Duration,
    pub sleep_interval: Duration,
    pub network_window: Duration,
    pub interaction_grace: Duration,
    pub terminate_grace: Duration,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            offline_grace: Duration::from_secs(120),
            idle_grace: Duration::from_secs(300),
            sleep_interval: Duration::from_secs(600),
            network_window: Duration::from_secs(60),
            interaction_grace: Duration::from_secs(300),
            terminate_grace: Duration::from_secs(20),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    Battery,
    Mains,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub now: Millis,
    pub power: Power,
    pub online: bool,
    pub idle_since: Option<Millis>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Arm,
    Disarm,
    Tick(Sample),
    Terminated(Millis),
    Resumed { sample: Sample, scheduled: bool },
    SuspendResolved,
    ActionFailed { action: Action, now: Millis },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TerminateUserSession { grace: Duration },
    ArmWake { after: Duration },
    CancelWake,
    Suspend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Teardown,
    Wake,
    Suspend,
}

impl Failure {
    pub fn as_str(self) -> &'static str {
        match self {
            Failure::Teardown => "the user session could not be verifiably terminated",
            Failure::Wake => "the wake alarm could not be armed",
            Failure::Suspend => "the machine refused to suspend",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Blocked {
    pub power_unknown: bool,
    pub idle_unobservable: bool,
}

impl Blocked {
    pub fn any(self) -> bool {
        self.power_unknown || self.idle_unobservable
    }

    pub fn describe(self) -> Option<&'static str> {
        match (self.power_unknown, self.idle_unobservable) {
            (true, true) => Some("mains state and input activity are both unreadable"),
            (true, false) => Some("mains state is unreadable"),
            (false, true) => Some("no readable input device, so idle cannot be proven"),
            (false, false) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Disarmed,
    Armed { offline_since: Option<Millis> },
    Terminating { attempts: u32, next_attempt: Millis },
    Probing { until: Millis },
    Asleep,
    Failed { failure: Failure, since: Millis },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Disarmed,
    Armed,
    Active,
    Failed,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Disarmed => "disarmed",
            Phase::Armed => "armed",
            Phase::Active => "active",
            Phase::Failed => "failed",
        }
    }
}

#[derive(Debug)]
pub struct Machine {
    thresholds: Thresholds,
    state: State,
    blocked: Blocked,
    sleep_failures: u32,
    // A queued suspend can outlive protocol state; retain its wake alarm until resume is observed.
    suspend_pending: bool,
}

impl Machine {
    pub fn new(thresholds: Thresholds) -> Self {
        Self {
            thresholds,
            state: State::Disarmed,
            blocked: Blocked::default(),
            sleep_failures: 0,
            suspend_pending: false,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn suspend_pending(&self) -> bool {
        self.suspend_pending
    }

    pub fn thresholds(&self) -> Thresholds {
        self.thresholds
    }

    pub fn blocked(&self) -> Blocked {
        self.blocked
    }

    pub fn phase(&self) -> Phase {
        match self.state {
            State::Disarmed => Phase::Disarmed,
            State::Armed { .. } => Phase::Armed,
            State::Terminating { .. } | State::Probing { .. } | State::Asleep => Phase::Active,
            State::Failed { .. } => Phase::Failed,
        }
    }

    pub fn tick_interval(&self) -> Duration {
        match self.state {
            State::Disarmed => Duration::from_secs(60),
            State::Armed { .. } => Duration::from_secs(15),
            State::Terminating { .. } => Duration::from_secs(1),
            State::Probing { .. } => Duration::from_secs(5),
            State::Asleep => Duration::from_secs(10),
            State::Failed { .. } => Duration::from_secs(15),
        }
    }

    pub fn handle(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Arm => self.arm(),
            Event::Disarm => self.disarm(),
            Event::Tick(sample) => self.tick(sample),
            Event::Terminated(now) => self.terminated(now),
            Event::Resumed { sample, scheduled } => self.resumed(sample, scheduled),
            Event::SuspendResolved => self.suspend_resolved(),
            Event::ActionFailed { action, now } => self.action_failed(action, now),
        }
    }

    fn arm(&mut self) -> Vec<Action> {
        if matches!(self.state, State::Disarmed) {
            self.state = State::Armed {
                offline_since: None,
            };
            self.blocked = Blocked::default();
            self.sleep_failures = 0;
        }
        Vec::new()
    }

    fn disarm(&mut self) -> Vec<Action> {
        if matches!(self.state, State::Disarmed) {
            return Vec::new();
        }
        self.stand_down()
    }

    fn stand_down(&mut self) -> Vec<Action> {
        let keep_alarm = self.suspend_pending;
        self.state = State::Disarmed;
        self.blocked = Blocked::default();
        self.sleep_failures = 0;
        if keep_alarm {
            Vec::new()
        } else {
            vec![Action::CancelWake]
        }
    }

    fn suspend_resolved(&mut self) -> Vec<Action> {
        if !self.suspend_pending {
            return Vec::new();
        }
        self.suspend_pending = false;
        match self.state {
            State::Disarmed | State::Failed { .. } => vec![Action::CancelWake],
            _ => Vec::new(),
        }
    }

    fn tick(&mut self, sample: Sample) -> Vec<Action> {
        match self.state {
            State::Disarmed | State::Asleep => Vec::new(),
            State::Armed { offline_since } => self.watch(sample, offline_since),
            State::Terminating {
                attempts,
                next_attempt,
            } => self.retry_teardown(sample, attempts, next_attempt),
            State::Probing { .. } => self.probe(sample),
            State::Failed { .. } => self.recover_only(sample),
        }
    }

    fn watch(&mut self, sample: Sample, offline_since: Option<Millis>) -> Vec<Action> {
        let offline_since = if sample.online {
            None
        } else {
            Some(offline_since.unwrap_or(sample.now))
        };
        self.state = State::Armed { offline_since };

        let mut blocked = Blocked::default();

        let on_battery = match sample.power {
            Power::Battery => true,
            Power::Mains => false,
            Power::Unknown => {
                blocked.power_unknown = true;
                false
            }
        };

        let idle_long_enough = match sample.idle_since {
            Some(since) => sample.now.saturating_sub(since) >= ms(self.thresholds.idle_grace),
            None => {
                blocked.idle_unobservable = true;
                false
            }
        };

        self.blocked = blocked;

        let offline_long_enough = offline_since.is_some_and(|since| {
            sample.now.saturating_sub(since) >= ms(self.thresholds.offline_grace)
        });

        if on_battery && offline_long_enough && idle_long_enough {
            return self.begin_teardown(sample.now);
        }

        Vec::new()
    }

    fn begin_teardown(&mut self, now: Millis) -> Vec<Action> {
        self.blocked = Blocked::default();
        self.state = State::Terminating {
            attempts: 1,
            next_attempt: now + ms(self.thresholds.terminate_grace),
        };
        vec![Action::TerminateUserSession {
            grace: self.thresholds.terminate_grace,
        }]
    }

    fn retry_teardown(
        &mut self,
        sample: Sample,
        attempts: u32,
        next_attempt: Millis,
    ) -> Vec<Action> {
        if sample.now < next_attempt {
            return Vec::new();
        }
        if attempts >= MAX_TEARDOWN_ATTEMPTS {
            self.state = State::Failed {
                failure: Failure::Teardown,
                since: sample.now,
            };
            return Vec::new();
        }
        self.state = State::Terminating {
            attempts: attempts + 1,
            next_attempt: sample.now + ms(self.thresholds.terminate_grace),
        };
        vec![Action::TerminateUserSession {
            grace: self.thresholds.terminate_grace,
        }]
    }

    fn terminated(&mut self, now: Millis) -> Vec<Action> {
        if matches!(self.state, State::Terminating { .. }) {
            self.begin_probing(now, self.thresholds.network_window);
        }
        Vec::new()
    }

    fn resumed(&mut self, sample: Sample, scheduled: bool) -> Vec<Action> {
        let resolved = self.suspend_resolved();
        if !matches!(self.state, State::Asleep | State::Probing { .. }) {
            return resolved;
        }
        self.sleep_failures = 0;

        let window = if scheduled {
            self.thresholds.network_window
        } else {
            self.thresholds.interaction_grace
        };

        self.begin_probing(sample.now, window);
        self.probe(sample)
    }

    fn action_failed(&mut self, action: Action, now: Millis) -> Vec<Action> {
        match action {
            Action::TerminateUserSession { .. } => self.teardown_failed(now),
            Action::ArmWake { .. } => self.sleep_failed(now, Failure::Wake),
            Action::Suspend => self.sleep_failed(now, Failure::Suspend),
            Action::CancelWake => Vec::new(),
        }
    }

    fn teardown_failed(&mut self, now: Millis) -> Vec<Action> {
        let State::Terminating { attempts, .. } = self.state else {
            return Vec::new();
        };
        if attempts >= MAX_TEARDOWN_ATTEMPTS {
            self.state = State::Failed {
                failure: Failure::Teardown,
                since: now,
            };
        } else {
            self.state = State::Terminating {
                attempts,
                next_attempt: now + ms(self.thresholds.terminate_grace),
            };
        }
        Vec::new()
    }

    fn sleep_failed(&mut self, now: Millis, failure: Failure) -> Vec<Action> {
        if !matches!(self.state, State::Asleep) {
            return Vec::new();
        }
        if matches!(failure, Failure::Wake) {
            self.suspend_pending = false;
        }
        self.sleep_failures += 1;

        if self.sleep_failures >= MAX_SLEEP_FAILURES {
            self.state = State::Failed {
                failure,
                since: now,
            };
        } else {
            self.state = State::Probing {
                until: now + ms(self.thresholds.network_window),
            };
        }
        Vec::new()
    }

    fn recover_only(&mut self, sample: Sample) -> Vec<Action> {
        if sample.online {
            return self.stand_down();
        }
        Vec::new()
    }

    fn begin_probing(&mut self, now: Millis, window: Duration) {
        self.state = State::Probing {
            until: now + ms(window),
        };
    }

    fn probe(&mut self, sample: Sample) -> Vec<Action> {
        let State::Probing { until } = self.state else {
            return Vec::new();
        };

        if sample.online {
            return self.stand_down();
        }

        if sample.now < until {
            return Vec::new();
        }

        let hold = ms(self.thresholds.interaction_grace);
        if let Some(since) = sample.idle_since {
            if sample.now.saturating_sub(since) < hold {
                self.state = State::Probing {
                    until: since + hold,
                };
                return Vec::new();
            }
        }

        self.state = State::Asleep;
        self.suspend_pending = true;
        vec![
            Action::ArmWake {
                after: self.thresholds.sleep_interval,
            },
            Action::Suspend,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEC: Millis = 1_000;
    const MIN: Millis = 60 * SEC;

    fn machine() -> Machine {
        Machine::new(Thresholds::default())
    }

    fn dark(now: Millis) -> Sample {
        Sample {
            now,
            power: Power::Battery,
            online: false,
            idle_since: Some(0),
        }
    }

    fn armed_at(now: Millis) -> Machine {
        let mut m = machine();
        m.handle(Event::Arm);
        m.handle(Event::Tick(Sample {
            online: true,
            ..dark(now)
        }));
        m
    }

    fn asleep() -> (Machine, Millis) {
        let mut m = armed_at(0);
        assert!(m.handle(Event::Tick(dark(10 * MIN))).is_empty());
        let actions = m.handle(Event::Tick(dark(12 * MIN)));
        assert_eq!(
            actions,
            vec![Action::TerminateUserSession {
                grace: Duration::from_secs(20)
            }]
        );
        m.handle(Event::Terminated(12 * MIN));
        let actions = m.handle(Event::Tick(dark(13 * MIN)));
        assert_eq!(
            actions,
            vec![
                Action::ArmWake {
                    after: Duration::from_secs(600)
                },
                Action::Suspend
            ]
        );
        assert_eq!(m.state(), State::Asleep);
        (m, 13 * MIN)
    }

    #[test]
    fn starts_disarmed_and_ignores_every_condition() {
        let mut m = machine();
        assert_eq!(m.phase(), Phase::Disarmed);
        assert!(m.handle(Event::Tick(dark(MIN))).is_empty());
        assert!(m.handle(Event::Tick(dark(60 * MIN))).is_empty());
        assert_eq!(m.state(), State::Disarmed);
    }

    #[test]
    fn arm_is_idempotent_and_keeps_the_streak() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(MIN)));
        let before = m.state();
        assert!(m.handle(Event::Arm).is_empty());
        assert_eq!(m.state(), before);
    }

    #[test]
    fn entry_needs_all_three_gates() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(Sample {
            power: Power::Mains,
            ..dark(10 * MIN)
        }));
        let actions = m.handle(Event::Tick(Sample {
            power: Power::Mains,
            ..dark(12 * MIN)
        }));
        assert!(actions.is_empty());
        assert_eq!(m.phase(), Phase::Armed);

        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(10 * MIN)));
        let actions = m.handle(Event::Tick(Sample {
            idle_since: Some(12 * MIN - 10 * SEC),
            ..dark(12 * MIN)
        }));
        assert!(actions.is_empty());
        assert_eq!(m.phase(), Phase::Armed);

        let mut m = armed_at(0);
        let actions = m.handle(Event::Tick(Sample {
            online: true,
            ..dark(12 * MIN)
        }));
        assert!(actions.is_empty());
        assert_eq!(m.phase(), Phase::Armed);
    }

    #[test]
    fn unknown_mains_state_blocks_entry_and_is_reported() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(Sample {
            power: Power::Unknown,
            ..dark(10 * MIN)
        }));
        let actions = m.handle(Event::Tick(Sample {
            power: Power::Unknown,
            ..dark(30 * MIN)
        }));
        assert!(
            actions.is_empty(),
            "an unreadable power tree is not evidence of a blackout"
        );
        assert_eq!(m.phase(), Phase::Armed);
        assert!(m.blocked().power_unknown);
        assert!(m.blocked().any());

        let actions = m.handle(Event::Tick(dark(31 * MIN)));
        assert_eq!(
            actions,
            vec![Action::TerminateUserSession {
                grace: Duration::from_secs(20)
            }]
        );
        assert!(!m.blocked().any());
    }

    #[test]
    fn unobservable_input_blocks_entry_and_is_reported() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(Sample {
            idle_since: None,
            ..dark(10 * MIN)
        }));
        let actions = m.handle(Event::Tick(Sample {
            idle_since: None,
            ..dark(30 * MIN)
        }));
        assert!(
            actions.is_empty(),
            "no readable input device means idle was never proven"
        );
        assert_eq!(m.phase(), Phase::Armed);
        assert!(m.blocked().idle_unobservable);
    }

    #[test]
    fn both_sensors_blocked_is_reported_together() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(Sample {
            power: Power::Unknown,
            idle_since: None,
            ..dark(30 * MIN)
        }));
        assert_eq!(
            m.blocked().describe(),
            Some("mains state and input activity are both unreadable")
        );
    }

    #[test]
    fn offline_streak_must_be_continuous() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(10 * MIN)));
        m.handle(Event::Tick(Sample {
            online: true,
            ..dark(11 * MIN)
        }));
        assert_eq!(
            m.state(),
            State::Armed {
                offline_since: None
            }
        );

        m.handle(Event::Tick(dark(11 * MIN + 30 * SEC)));
        let actions = m.handle(Event::Tick(dark(13 * MIN)));
        assert!(actions.is_empty());

        let actions = m.handle(Event::Tick(dark(13 * MIN + 31 * SEC)));
        assert_eq!(
            actions,
            vec![Action::TerminateUserSession {
                grace: Duration::from_secs(20)
            }]
        );
    }

    #[test]
    fn entry_terminates_then_probes_before_sleeping() {
        let (m, _) = asleep();
        assert_eq!(m.phase(), Phase::Active);
    }

    #[test]
    fn a_teardown_that_never_reports_is_retried_then_declared_failed() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(10 * MIN)));
        let first = m.handle(Event::Tick(dark(12 * MIN)));
        assert_eq!(first.len(), 1);
        assert_eq!(
            m.state(),
            State::Terminating {
                attempts: 1,
                next_attempt: 12 * MIN + 20 * SEC
            }
        );

        let second = m.handle(Event::Tick(dark(12 * MIN + 20 * SEC)));
        assert_eq!(
            second,
            vec![Action::TerminateUserSession {
                grace: Duration::from_secs(20)
            }]
        );
        let third = m.handle(Event::Tick(dark(12 * MIN + 40 * SEC)));
        assert_eq!(third.len(), 1);

        let after = m.handle(Event::Tick(dark(12 * MIN + 60 * SEC)));
        assert!(after.is_empty());
        assert!(matches!(
            m.state(),
            State::Failed {
                failure: Failure::Teardown,
                ..
            }
        ));
        assert_eq!(m.phase(), Phase::Failed);
    }

    #[test]
    fn a_failed_teardown_never_reaches_asleep() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(10 * MIN)));
        let mut now = 12 * MIN;
        m.handle(Event::Tick(dark(now)));

        for _ in 0..MAX_TEARDOWN_ATTEMPTS {
            m.handle(Event::ActionFailed {
                action: Action::TerminateUserSession {
                    grace: Duration::from_secs(20),
                },
                now,
            });
            now += 20 * SEC;
            m.handle(Event::Tick(dark(now)));
        }

        assert!(matches!(
            m.state(),
            State::Failed {
                failure: Failure::Teardown,
                ..
            }
        ));
        for minute in 20..80 {
            assert!(m.handle(Event::Tick(dark(minute * MIN))).is_empty());
        }
        assert_eq!(m.phase(), Phase::Failed);
    }

    #[test]
    fn a_failed_protocol_still_recovers_when_the_link_returns() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(10 * MIN)));
        m.handle(Event::Tick(dark(12 * MIN)));
        let mut now = 12 * MIN;
        for _ in 0..MAX_TEARDOWN_ATTEMPTS {
            m.handle(Event::ActionFailed {
                action: Action::TerminateUserSession {
                    grace: Duration::from_secs(20),
                },
                now,
            });
            now += 20 * SEC;
            m.handle(Event::Tick(dark(now)));
        }
        assert_eq!(m.phase(), Phase::Failed);

        let actions = m.handle(Event::Tick(Sample {
            online: true,
            ..dark(now + MIN)
        }));
        assert_eq!(actions, vec![Action::CancelWake]);
        assert_eq!(m.state(), State::Disarmed);
    }

    #[test]
    fn a_failed_wake_alarm_keeps_the_machine_awake_instead_of_suspending() {
        let (mut m, slept_at) = asleep();
        let actions = m.handle(Event::ActionFailed {
            action: Action::ArmWake {
                after: Duration::from_secs(600),
            },
            now: slept_at,
        });
        assert!(
            actions.is_empty(),
            "nothing to undo: the alarm was never armed"
        );
        assert_eq!(
            m.state(),
            State::Probing {
                until: slept_at + 60 * SEC
            },
            "a machine that cannot wake itself must not go down"
        );
    }

    #[test]
    fn a_refused_suspend_keeps_the_alarm_and_stays_awake() {
        let (mut m, slept_at) = asleep();
        let actions = m.handle(Event::ActionFailed {
            action: Action::Suspend,
            now: slept_at,
        });
        assert!(
            actions.is_empty(),
            "the alarm is harmless on an awake machine, and cancelling it \
             would strand a suspend that lands after we gave up"
        );
        assert_eq!(
            m.state(),
            State::Probing {
                until: slept_at + 60 * SEC
            }
        );
        assert_ne!(m.state(), State::Asleep);
    }

    #[test]
    fn disarming_a_queued_suspend_leaves_its_alarm_armed() {
        let (mut m, _) = asleep();
        assert!(
            m.handle(Event::Disarm).is_empty(),
            "the only thing that can bring a suspending machine back must survive the disarm"
        );
        assert_eq!(m.state(), State::Disarmed);

        assert!(m.handle(Event::Tick(dark(90 * MIN))).is_empty());
        assert_eq!(
            m.handle(Event::Resumed {
                sample: dark(95 * MIN),
                scheduled: true,
            }),
            vec![Action::CancelWake],
            "the suspend has now happened, so the alarm kept to bound it has \
             done its job and must not be left armed for the next lid close"
        );
        assert_eq!(m.state(), State::Disarmed);
        assert!(!m.suspend_pending());
    }

    #[test]
    fn a_disarm_racing_a_manual_wake_clears_the_alarm_either_way() {
        for disarm_first in [true, false] {
            let (mut m, slept_at) = asleep();
            let resume = Event::Resumed {
                sample: dark(slept_at + 2 * MIN),
                scheduled: false,
            };

            let actions = if disarm_first {
                let first = m.handle(Event::Disarm);
                assert!(first.is_empty(), "the suspend was still outstanding");
                m.handle(Event::SuspendResolved)
            } else {
                m.handle(Event::SuspendResolved);
                m.handle(resume);
                m.handle(Event::Disarm)
            };

            assert_eq!(
                actions,
                vec![Action::CancelWake],
                "disarm_first = {disarm_first}"
            );
            assert_eq!(m.state(), State::Disarmed);
            assert!(!m.suspend_pending());
        }
    }

    #[test]
    fn a_resolved_suspend_releases_the_alarm_a_degraded_protocol_was_holding() {
        let (mut m, slept_at) = asleep();
        for _ in 0..MAX_SLEEP_FAILURES {
            m.handle(Event::ActionFailed {
                action: Action::Suspend,
                now: slept_at,
            });
            m.handle(Event::Tick(dark(slept_at + 60 * SEC)));
        }
        assert_eq!(m.phase(), Phase::Failed);
        assert!(m.suspend_pending());

        assert_eq!(m.handle(Event::SuspendResolved), vec![Action::CancelWake]);
        assert_eq!(m.phase(), Phase::Failed);
    }

    #[test]
    fn a_suspend_written_off_by_the_watchdog_is_still_pending_in_probing() {
        let (mut m, slept_at) = asleep();
        m.handle(Event::ActionFailed {
            action: Action::Suspend,
            now: slept_at,
        });
        assert!(matches!(m.state(), State::Probing { .. }));
        assert!(
            m.suspend_pending(),
            "giving up on a suspend is a decision about our patience, not \
             evidence that logind dropped the job"
        );

        assert!(m.handle(Event::Disarm).is_empty());
        assert_eq!(m.state(), State::Disarmed);
        assert!(m.suspend_pending());
    }

    #[test]
    fn the_link_returning_after_a_watchdog_also_keeps_the_alarm() {
        let (mut m, slept_at) = asleep();
        m.handle(Event::ActionFailed {
            action: Action::Suspend,
            now: slept_at,
        });

        let actions = m.handle(Event::Tick(Sample {
            online: true,
            ..dark(slept_at + 10 * SEC)
        }));
        assert!(
            actions.is_empty(),
            "recovery ends the protocol, but it cannot prove the queued \
             suspend went away"
        );
        assert_eq!(m.state(), State::Disarmed);
    }

    #[test]
    fn a_resume_resolves_the_suspend_even_with_the_protocol_over() {
        let (mut m, slept_at) = asleep();
        m.handle(Event::ActionFailed {
            action: Action::Suspend,
            now: slept_at,
        });
        m.handle(Event::Disarm);
        assert!(m.suspend_pending());

        assert_eq!(m.handle(Event::SuspendResolved), vec![Action::CancelWake]);
        assert!(!m.suspend_pending());

        assert!(m.handle(Event::SuspendResolved).is_empty());

        m.handle(Event::Arm);
        assert_eq!(m.handle(Event::Disarm), vec![Action::CancelWake]);
    }

    #[test]
    fn a_wake_alarm_that_never_armed_leaves_nothing_outstanding() {
        let (mut m, slept_at) = asleep();
        m.handle(Event::ActionFailed {
            action: Action::ArmWake {
                after: Duration::from_secs(600),
            },
            now: slept_at,
        });
        assert!(
            !m.suspend_pending(),
            "the batch was abandoned before Suspend, so logind never saw one"
        );
        assert_eq!(m.handle(Event::Disarm), vec![Action::CancelWake]);
    }

    #[test]
    fn disarming_an_awake_protocol_does_cancel_the_alarm() {
        let (mut m, slept_at) = asleep();
        m.handle(Event::Resumed {
            sample: dark(slept_at + 10 * MIN),
            scheduled: true,
        });
        assert_eq!(
            m.handle(Event::Disarm),
            vec![Action::CancelWake],
            "with nothing in flight there is no reason to keep a wake pending"
        );
    }

    #[test]
    fn repeated_sleep_failures_give_up_instead_of_looping() {
        let (mut m, mut now) = asleep();
        for _ in 0..MAX_SLEEP_FAILURES - 1 {
            m.handle(Event::ActionFailed {
                action: Action::ArmWake {
                    after: Duration::from_secs(600),
                },
                now,
            });
            assert!(matches!(m.state(), State::Probing { .. }));
            now += 60 * SEC;
            let actions = m.handle(Event::Tick(dark(now)));
            assert_eq!(actions.len(), 2);
        }
        m.handle(Event::ActionFailed {
            action: Action::ArmWake {
                after: Duration::from_secs(600),
            },
            now,
        });
        assert!(matches!(
            m.state(),
            State::Failed {
                failure: Failure::Wake,
                ..
            }
        ));
    }

    #[test]
    fn a_successful_sleep_cycle_clears_earlier_failures() {
        let (mut m, mut now) = asleep();
        m.handle(Event::ActionFailed {
            action: Action::ArmWake {
                after: Duration::from_secs(600),
            },
            now,
        });
        now += 60 * SEC;
        m.handle(Event::Tick(dark(now)));
        assert_eq!(m.state(), State::Asleep);

        now += 10 * MIN;
        m.handle(Event::Resumed {
            sample: dark(now),
            scheduled: true,
        });
        now += 60 * SEC;
        m.handle(Event::Tick(dark(now)));
        for _ in 0..MAX_SLEEP_FAILURES - 1 {
            m.handle(Event::ActionFailed {
                action: Action::ArmWake {
                    after: Duration::from_secs(600),
                },
                now,
            });
            now += 60 * SEC;
            m.handle(Event::Tick(dark(now)));
        }
        assert_eq!(m.state(), State::Asleep);
    }

    #[test]
    fn a_failure_reported_outside_its_state_is_ignored() {
        let mut m = armed_at(0);
        assert!(m
            .handle(Event::ActionFailed {
                action: Action::Suspend,
                now: MIN,
            })
            .is_empty());
        assert_eq!(m.phase(), Phase::Armed);
    }

    #[test]
    fn scheduled_wake_that_finds_no_link_sleeps_again_without_an_idle_penalty() {
        let (mut m, slept_at) = asleep();
        let now = slept_at + 10 * MIN;
        let actions = m.handle(Event::Resumed {
            sample: dark(now),
            scheduled: true,
        });
        assert!(actions.is_empty(), "the link window must be honoured first");
        assert_eq!(
            m.state(),
            State::Probing {
                until: now + 60 * SEC
            }
        );

        let actions = m.handle(Event::Tick(dark(now + 60 * SEC)));
        assert_eq!(
            actions,
            vec![
                Action::ArmWake {
                    after: Duration::from_secs(600)
                },
                Action::Suspend
            ]
        );
    }

    #[test]
    fn unscheduled_wake_holds_awake_long_enough_to_log_in() {
        let (mut m, slept_at) = asleep();
        let now = slept_at + 3 * MIN;
        m.handle(Event::Resumed {
            sample: dark(now),
            scheduled: false,
        });
        assert_eq!(
            m.state(),
            State::Probing {
                until: now + 5 * MIN
            },
            "a hand-driven wake gets the interaction window, not the link window"
        );

        assert!(m.handle(Event::Tick(dark(now + 90 * SEC))).is_empty());
        assert_eq!(m.phase(), Phase::Active);
    }

    #[test]
    fn typing_at_the_console_keeps_pushing_the_suspend_out() {
        let (mut m, slept_at) = asleep();
        let now = slept_at + 10 * MIN;
        m.handle(Event::Resumed {
            sample: dark(now),
            scheduled: true,
        });

        let typed_at = now + 30 * SEC;
        assert!(m
            .handle(Event::Tick(Sample {
                idle_since: Some(typed_at),
                ..dark(now + 61 * SEC)
            }))
            .is_empty());
        assert_eq!(
            m.state(),
            State::Probing {
                until: typed_at + 5 * MIN
            }
        );

        assert!(m
            .handle(Event::Tick(Sample {
                idle_since: Some(typed_at),
                ..dark(typed_at + 4 * MIN)
            }))
            .is_empty());

        let actions = m.handle(Event::Tick(Sample {
            idle_since: Some(typed_at),
            ..dark(typed_at + 5 * MIN)
        }));
        assert_eq!(
            actions,
            vec![
                Action::ArmWake {
                    after: Duration::from_secs(600)
                },
                Action::Suspend
            ]
        );
    }

    #[test]
    fn internet_alone_recovers_and_returns_to_disarmed() {
        let (mut m, slept_at) = asleep();
        let now = slept_at + 10 * MIN;
        let actions = m.handle(Event::Resumed {
            sample: Sample {
                online: true,
                ..dark(now)
            },
            scheduled: true,
        });
        assert_eq!(actions, vec![Action::CancelWake]);
        assert_eq!(m.state(), State::Disarmed);
        assert_eq!(m.phase(), Phase::Disarmed);
    }

    #[test]
    fn recovery_is_one_shot_and_does_not_re_enter() {
        let (mut m, slept_at) = asleep();
        m.handle(Event::Resumed {
            sample: Sample {
                online: true,
                ..dark(slept_at + 10 * MIN)
            },
            scheduled: true,
        });
        let actions = m.handle(Event::Tick(dark(70 * MIN)));
        assert!(actions.is_empty());
        assert_eq!(m.state(), State::Disarmed);
    }

    #[test]
    fn mains_returning_does_not_end_an_active_protocol() {
        let (mut m, slept_at) = asleep();
        let now = slept_at + 10 * MIN;
        m.handle(Event::Resumed {
            sample: Sample {
                power: Power::Mains,
                ..dark(now)
            },
            scheduled: true,
        });
        let actions = m.handle(Event::Tick(Sample {
            power: Power::Mains,
            ..dark(now + 60 * SEC)
        }));
        assert_eq!(
            actions,
            vec![
                Action::ArmWake {
                    after: Duration::from_secs(600)
                },
                Action::Suspend
            ]
        );
        assert_eq!(m.phase(), Phase::Active);
    }

    #[test]
    fn disarm_from_armed_cancels_and_stops_watching() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(10 * MIN)));
        assert_eq!(m.handle(Event::Disarm), vec![Action::CancelWake]);
        assert_eq!(m.state(), State::Disarmed);
        assert!(m.handle(Event::Tick(dark(60 * MIN))).is_empty());
    }

    #[test]
    fn disarm_while_active_cancels_the_pending_wake() {
        let (mut m, slept_at) = asleep();
        m.handle(Event::Resumed {
            sample: dark(slept_at + 10 * MIN),
            scheduled: true,
        });
        assert_eq!(m.handle(Event::Disarm), vec![Action::CancelWake]);
        assert_eq!(m.state(), State::Disarmed);

        assert!(m.handle(Event::Tick(dark(90 * MIN))).is_empty());
    }

    #[test]
    fn disarm_while_asleep_is_accepted() {
        let (mut m, _) = asleep();
        assert!(m.handle(Event::Disarm).is_empty());
        assert_eq!(m.state(), State::Disarmed);
    }

    #[test]
    fn disarm_out_of_a_failed_protocol_is_accepted() {
        let mut m = armed_at(0);
        m.handle(Event::Tick(dark(10 * MIN)));
        m.handle(Event::Tick(dark(12 * MIN)));
        let mut now = 12 * MIN;
        for _ in 0..MAX_TEARDOWN_ATTEMPTS {
            m.handle(Event::ActionFailed {
                action: Action::TerminateUserSession {
                    grace: Duration::from_secs(20),
                },
                now,
            });
            now += 20 * SEC;
            m.handle(Event::Tick(dark(now)));
        }
        assert_eq!(m.phase(), Phase::Failed);
        assert_eq!(m.handle(Event::Disarm), vec![Action::CancelWake]);
        assert_eq!(m.state(), State::Disarmed);
    }

    #[test]
    fn disarm_when_already_disarmed_does_nothing() {
        let mut m = machine();
        assert!(m.handle(Event::Disarm).is_empty());
        assert_eq!(m.state(), State::Disarmed);
    }

    #[test]
    fn a_resume_without_an_active_protocol_is_ignored() {
        let mut m = armed_at(0);
        assert!(m
            .handle(Event::Resumed {
                sample: dark(5 * MIN),
                scheduled: false,
            })
            .is_empty());
        assert_eq!(m.phase(), Phase::Armed);
    }

    #[test]
    fn tick_cadence_tightens_with_the_state() {
        let mut m = machine();
        assert_eq!(m.tick_interval(), Duration::from_secs(60));
        m.handle(Event::Arm);
        assert_eq!(m.tick_interval(), Duration::from_secs(15));
        m.handle(Event::Tick(dark(10 * MIN)));
        m.handle(Event::Tick(dark(12 * MIN)));
        assert_eq!(m.tick_interval(), Duration::from_secs(1));
        m.handle(Event::Terminated(12 * MIN));
        assert_eq!(m.tick_interval(), Duration::from_secs(5));
    }
}

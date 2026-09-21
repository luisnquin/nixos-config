use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;
use std::time::Duration;

use outage::effects::Effects;
use outage::machine::{
    Event, Failure, Machine, Millis, Phase, Power, Sample, State, Thresholds, MAX_TEARDOWN_ATTEMPTS,
};
use outage::runner::Runner;

const SEC: Millis = 1_000;
const MIN: Millis = 60 * SEC;
const GRACE: Duration = Duration::from_secs(20);
const INTERVAL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Did {
    Terminated { grace: Duration },
    ArmedWake { after: Duration },
    CancelledWake,
    Suspended,
}

#[derive(Default)]
struct Journal {
    entries: Vec<Did>,
    terminate_fails: bool,
}

#[derive(Clone, Default)]
struct Recorder {
    journal: Rc<RefCell<Journal>>,
    clock: Rc<Cell<Millis>>,
}

impl Recorder {
    fn entries(&self) -> Vec<Did> {
        self.journal.borrow().entries.clone()
    }

    fn drain(&self) -> Vec<Did> {
        std::mem::take(&mut self.journal.borrow_mut().entries)
    }

    fn fail_termination(&self) {
        self.journal.borrow_mut().terminate_fails = true;
    }

    fn push(&self, did: Did) {
        self.journal.borrow_mut().entries.push(did);
    }
}

impl Effects for Recorder {
    fn now(&self) -> Millis {
        self.clock.get()
    }

    fn terminate_user_session(&mut self, grace: Duration) -> io::Result<()> {
        self.push(Did::Terminated { grace });
        if self.journal.borrow().terminate_fails {
            return Err(io::Error::other("logind refused"));
        }
        Ok(())
    }

    fn arm_wake(&mut self, after: Duration) -> io::Result<()> {
        self.push(Did::ArmedWake { after });
        Ok(())
    }

    fn cancel_wake(&mut self) -> io::Result<()> {
        self.push(Did::CancelledWake);
        Ok(())
    }

    fn suspend(&mut self) -> io::Result<()> {
        self.push(Did::Suspended);
        Ok(())
    }
}

struct Harness {
    runner: Runner<Recorder>,
    log: Recorder,
}

impl Harness {
    fn new() -> Self {
        let log = Recorder::default();
        Self {
            runner: Runner::new(Machine::new(Thresholds::default()), log.clone()),
            log,
        }
    }

    fn tick(&mut self, sample: Sample) -> State {
        self.log.clock.set(sample.now);
        self.runner.dispatch(Event::Tick(sample));
        if self.runner.last_dispatch_terminated() {
            self.runner.dispatch(Event::Terminated(sample.now));
        }
        self.runner.state()
    }

    fn resume(&mut self, sample: Sample, scheduled: bool) {
        self.log.clock.set(sample.now);
        self.runner.dispatch(Event::Resumed { sample, scheduled });
    }

    fn at(&mut self, now: Millis, event: Event) {
        self.log.clock.set(now);
        self.runner.dispatch(event);
    }
}

fn dark(now: Millis) -> Sample {
    Sample {
        now,
        power: Power::Battery,
        online: false,
        idle_since: Some(0),
    }
}

fn lit(now: Millis) -> Sample {
    Sample {
        online: true,
        ..dark(now)
    }
}

#[test]
fn a_disarmed_controller_ignores_a_full_blackout() {
    let mut h = Harness::new();
    for minute in 0..30 {
        h.tick(dark(minute * MIN));
    }
    assert_eq!(h.runner.phase(), Phase::Disarmed);
    assert!(
        h.log.entries().is_empty(),
        "nothing may happen without an explicit arm"
    );
}

#[test]
fn the_whole_outage_lifecycle_runs_end_to_end() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);
    assert_eq!(h.runner.phase(), Phase::Armed);

    h.tick(lit(10 * MIN));
    assert!(h.log.entries().is_empty());

    h.tick(dark(11 * MIN));
    h.tick(dark(12 * MIN - SEC));
    assert!(h.log.entries().is_empty());
    assert_eq!(h.runner.phase(), Phase::Armed);

    h.tick(dark(13 * MIN));
    assert_eq!(h.log.drain(), vec![Did::Terminated { grace: GRACE }]);
    assert_eq!(h.runner.phase(), Phase::Active);

    h.tick(dark(14 * MIN));
    assert_eq!(
        h.log.drain(),
        vec![Did::ArmedWake { after: INTERVAL }, Did::Suspended],
        "the wake must be armed before the suspend, never after"
    );
    assert_eq!(h.runner.state(), State::Asleep);

    h.resume(dark(24 * MIN), true);
    assert!(h.log.entries().is_empty());
    h.tick(dark(25 * MIN));
    assert_eq!(
        h.log.drain(),
        vec![Did::ArmedWake { after: INTERVAL }, Did::Suspended]
    );

    h.resume(lit(35 * MIN), true);
    assert_eq!(h.log.drain(), vec![Did::CancelledWake]);
    assert_eq!(h.runner.phase(), Phase::Disarmed);

    h.tick(dark(60 * MIN));
    h.tick(dark(90 * MIN));
    assert!(h.log.entries().is_empty());
}

#[test]
fn recovery_never_suspends_again_and_leaves_no_alarm() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.tick(dark(13 * MIN));
    h.tick(dark(14 * MIN));
    h.log.drain();

    h.resume(lit(24 * MIN), true);
    assert_eq!(h.log.drain(), vec![Did::CancelledWake]);

    for minute in 25..60 {
        h.tick(dark(minute * MIN));
    }
    assert!(
        h.log.entries().is_empty(),
        "a recovered controller must stay awake and disarmed"
    );
}

#[test]
fn mains_coming_back_does_not_end_a_running_protocol() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.tick(dark(13 * MIN));
    h.log.drain();

    let plugged = |now: Millis| Sample {
        power: Power::Mains,
        ..dark(now)
    };
    h.tick(plugged(14 * MIN));
    assert_eq!(
        h.log.drain(),
        vec![Did::ArmedWake { after: INTERVAL }, Did::Suspended]
    );
    assert_eq!(h.runner.phase(), Phase::Active);
}

#[test]
fn an_unreadable_power_tree_never_authorises_an_entry() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);

    for step in 0..60 {
        let now = step * 30 * SEC;
        h.tick(Sample {
            power: Power::Unknown,
            ..dark(now)
        });
    }
    assert!(h.log.entries().is_empty());
    assert_eq!(h.runner.phase(), Phase::Armed);
    assert!(h.runner.machine().blocked().power_unknown);

    h.tick(dark(31 * MIN));
    assert_eq!(h.log.drain(), vec![Did::Terminated { grace: GRACE }]);
}

#[test]
fn a_console_login_during_the_protocol_can_disarm_without_racing_to_sleep() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.tick(dark(13 * MIN));
    h.tick(dark(14 * MIN));
    h.log.drain();
    assert_eq!(h.runner.state(), State::Asleep);

    let woke = 24 * MIN;
    h.resume(dark(woke), false);
    assert!(h.log.entries().is_empty());

    for step in 1..=4 {
        let now = woke + step * 30 * SEC;
        h.tick(Sample {
            idle_since: Some(now),
            ..dark(now)
        });
    }
    assert!(
        h.log.entries().is_empty(),
        "typing at the console must hold the protocol awake"
    );

    h.at(woke + 3 * MIN, Event::Disarm);
    assert_eq!(h.log.drain(), vec![Did::CancelledWake]);
    assert_eq!(h.runner.phase(), Phase::Disarmed);

    for minute in 30..60 {
        h.tick(dark(minute * MIN));
    }
    assert!(h.log.entries().is_empty());
}

#[test]
fn a_refused_teardown_never_lets_the_protocol_sleep() {
    let mut h = Harness::new();
    h.log.fail_termination();
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.tick(dark(13 * MIN));
    assert_eq!(h.log.drain(), vec![Did::Terminated { grace: GRACE }]);

    let mut now = 13 * MIN;
    for _ in 1..MAX_TEARDOWN_ATTEMPTS {
        now += 20 * SEC;
        h.tick(dark(now));
        assert_eq!(h.log.drain(), vec![Did::Terminated { grace: GRACE }]);
    }

    now += 20 * SEC;
    h.tick(dark(now));
    assert!(matches!(
        h.runner.state(),
        State::Failed {
            failure: Failure::Teardown,
            ..
        }
    ));

    for minute in 20..120 {
        h.tick(dark(minute * MIN));
    }
    assert!(
        h.log.entries().is_empty(),
        "a protocol that could not clear the session must never suspend"
    );
    assert_eq!(h.runner.phase(), Phase::Failed);

    h.tick(lit(121 * MIN));
    assert_eq!(h.log.drain(), vec![Did::CancelledWake]);
    assert_eq!(h.runner.phase(), Phase::Disarmed);
}

#[test]
fn a_teardown_is_acknowledged_only_once_it_is_verified() {
    let mut h = Harness::new();
    h.log.fail_termination();
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.tick(dark(13 * MIN));

    assert!(
        matches!(h.runner.state(), State::Terminating { .. }),
        "a teardown that did not report a cleared cgroup is not a teardown"
    );
    assert!(!h.runner.last_dispatch_terminated());
    assert!(
        h.runner.health().is_some(),
        "the failure has to survive for `outage status` to report"
    );
}

#[test]
fn arming_twice_does_not_double_terminate() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.tick(dark(13 * MIN));
    h.at(13 * MIN, Event::Arm);
    h.tick(dark(13 * MIN + SEC));

    assert_eq!(
        h.log.entries(),
        vec![Did::Terminated { grace: GRACE }],
        "the session is torn down exactly once"
    );
}

#[test]
fn disarming_before_entry_prevents_it_entirely() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.at(10 * MIN, Event::Disarm);
    h.log.drain();

    h.tick(dark(20 * MIN));
    h.tick(dark(30 * MIN));
    assert!(
        h.log.entries().is_empty(),
        "a disarmed controller must never terminate a session"
    );
}

#[test]
fn a_disarm_on_a_queued_suspend_keeps_the_only_way_back() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);
    h.tick(dark(10 * MIN));
    h.tick(dark(13 * MIN));
    h.tick(dark(14 * MIN));
    h.log.drain();
    assert_eq!(h.runner.state(), State::Asleep);

    h.at(14 * MIN + SEC, Event::Disarm);
    assert!(
        h.log.entries().is_empty(),
        "the wake alarm must outlive a disarm that races a pending suspend"
    );
    assert_eq!(h.runner.phase(), Phase::Disarmed);
    assert!(h.runner.machine().suspend_pending());

    h.at(24 * MIN, Event::SuspendResolved);
    assert_eq!(h.log.drain(), vec![Did::CancelledWake]);
    assert!(!h.runner.machine().suspend_pending());

    h.resume(dark(24 * MIN), true);
    assert!(h.log.entries().is_empty());
    assert_eq!(h.runner.phase(), Phase::Disarmed);

    for minute in 25..60 {
        h.tick(dark(minute * MIN));
    }
    assert!(h.log.entries().is_empty());
}

#[test]
fn a_user_still_working_in_the_dark_is_never_interrupted() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);

    for step in 0..60 {
        let now = step * 30 * SEC;
        h.tick(Sample {
            idle_since: Some(now),
            ..dark(now)
        });
    }
    assert!(h.log.entries().is_empty());
    assert_eq!(h.runner.phase(), Phase::Armed);

    let left = 30 * MIN;
    h.tick(Sample {
        idle_since: Some(left),
        ..dark(left + 4 * MIN)
    });
    assert!(h.log.entries().is_empty());
    h.tick(Sample {
        idle_since: Some(left),
        ..dark(left + 5 * MIN)
    });
    assert_eq!(h.log.drain(), vec![Did::Terminated { grace: GRACE }]);
}

#[test]
fn an_unwatched_keyboard_is_never_mistaken_for_an_absent_user() {
    let mut h = Harness::new();
    h.at(0, Event::Arm);

    for step in 0..120 {
        let now = step * 30 * SEC;
        h.tick(Sample {
            idle_since: None,
            ..dark(now)
        });
    }
    assert!(
        h.log.entries().is_empty(),
        "unmonitored is not idle; someone may be sitting right at the keyboard"
    );
    assert!(h.runner.machine().blocked().idle_unobservable);
    assert_eq!(h.runner.phase(), Phase::Armed);
}

use std::time::Duration;

use crate::control::{Reply, Request};
use crate::effects::Effects;
use crate::machine::{
    Action, Blocked, Event, Millis, Phase, Power, Sample, State, MAX_TEARDOWN_ATTEMPTS,
};
use crate::runner::Runner;

pub const PROBE_WAIT_MS: Millis = 200;

pub const SUSPEND_WATCHDOG: Duration = Duration::from_secs(90);

pub trait World {
    fn take_requests(&mut self) -> Vec<Request>;
    fn answer(&mut self, replies: Vec<Reply>);
    fn take_resume(&mut self) -> Option<bool>;
    fn refresh(&mut self);
    fn idle_since(&self) -> Option<Millis>;
    fn power(&mut self) -> Power;
    fn connectivity(&mut self) -> Option<(Millis, bool)>;
    fn request_probe(&mut self);
    fn probe_inflight(&self) -> bool;
    fn rescan_input(&mut self);
    fn awake_ms(&self) -> Millis;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    Tick,
    Resumed { scheduled: bool },
}

pub struct Driver {
    next_tick: Millis,
    max_probe_age: Millis,
    pending_resume: Option<bool>,
    suspend_watch: Option<Millis>,
    watchdog: Millis,
    blocked: Blocked,
}

impl Driver {
    pub fn new(now: Millis, max_probe_age: Duration) -> Self {
        Self {
            next_tick: now,
            max_probe_age: max_probe_age.as_millis() as Millis,
            pending_resume: None,
            suspend_watch: None,
            watchdog: SUSPEND_WATCHDOG.as_millis() as Millis,
            blocked: Blocked::default(),
        }
    }

    pub fn step<W: World, E: Effects>(&mut self, world: &mut W, runner: &mut Runner<E>) -> Millis {
        world.refresh();

        if let Some(scheduled) = world.take_resume() {
            self.suspend_watch = None;
            runner.dispatch(Event::SuspendResolved);
            self.pending_resume = Some(self.pending_resume.unwrap_or(false) || scheduled);
        }

        self.serve_control(world, runner);

        if self.pending_resume.is_some() {
            if !matches!(runner.state(), State::Asleep | State::Probing { .. }) {
                self.pending_resume = None;
            } else {
                let now = runner.effects().now();
                let Some(online) = self.fresh_connectivity(world, now) else {
                    return PROBE_WAIT_MS;
                };
                let scheduled = self.pending_resume.take().unwrap_or(false);
                self.dispatch_sampled(world, runner, Pending::Resumed { scheduled }, online);
                self.next_tick = runner.effects().now();
                return self.settle(world, runner);
            }
        }

        if self.suspend_overdue(world, runner) {
            crate::log(&format!(
                "still awake {}s after the suspend was issued; treating it as refused",
                self.watchdog / 1_000
            ));
            let now = runner.effects().now();
            runner.dispatch(Event::ActionFailed {
                action: Action::Suspend,
                now,
            });
            self.next_tick = runner.effects().now();
            return self.settle(world, runner);
        }

        if runner.effects().now() < self.next_tick {
            return self.settle(world, runner);
        }

        world.rescan_input();

        if matches!(runner.state(), State::Disarmed | State::Asleep) {
            self.next_tick = runner.effects().now() + tick_ms(runner);
            return self.settle(world, runner);
        }

        let now = runner.effects().now();
        let Some(online) = self.fresh_connectivity(world, now) else {
            return PROBE_WAIT_MS;
        };

        self.dispatch_sampled(world, runner, Pending::Tick, online);
        self.next_tick = runner.effects().now() + tick_ms(runner);
        self.settle(world, runner)
    }

    fn suspend_overdue<W: World, E: Effects>(&self, world: &W, runner: &Runner<E>) -> bool {
        if !matches!(runner.state(), State::Asleep) {
            return false;
        }
        self.suspend_watch
            .is_some_and(|since| world.awake_ms().saturating_sub(since) >= self.watchdog)
    }

    fn settle<W: World, E: Effects>(&mut self, world: &W, runner: &Runner<E>) -> Millis {
        if matches!(runner.state(), State::Asleep) {
            self.suspend_watch.get_or_insert_with(|| world.awake_ms());
        } else {
            self.suspend_watch = None;
        }

        let blocked = runner.machine().blocked();
        if blocked != self.blocked {
            match blocked.describe() {
                Some(why) => crate::log(&format!("entry is blocked: {why}")),
                None if self.blocked.any() => crate::log("entry is no longer blocked"),
                None => {}
            }
            self.blocked = blocked;
        }

        self.wait(world, runner)
    }

    fn dispatch_sampled<W: World, E: Effects>(
        &mut self,
        world: &mut W,
        runner: &mut Runner<E>,
        kind: Pending,
        online: bool,
    ) {
        let before = runner.state();
        world.refresh();
        self.serve_control(world, runner);
        if runner.state() != before {
            crate::log("a control request overtook a pending transition; sample dropped");
            return;
        }

        let sample = Sample {
            now: runner.effects().now(),
            power: world.power(),
            online,
            idle_since: world.idle_since(),
        };
        let event = match kind {
            Pending::Tick => Event::Tick(sample),
            Pending::Resumed { scheduled } => Event::Resumed { sample, scheduled },
        };
        runner.dispatch(event);

        if runner.last_dispatch_terminated() {
            let now = runner.effects().now();
            runner.dispatch(Event::Terminated(now));
        }
    }

    fn fresh_connectivity<W: World>(&mut self, world: &mut W, now: Millis) -> Option<bool> {
        if let Some((at, online)) = world.connectivity() {
            if now.saturating_sub(at) <= self.max_probe_age {
                return Some(online);
            }
        }
        world.request_probe();
        None
    }

    fn wait<W: World, E: Effects>(&self, world: &W, runner: &Runner<E>) -> Millis {
        if world.probe_inflight() {
            return PROBE_WAIT_MS;
        }
        self.next_tick.saturating_sub(runner.effects().now())
    }

    fn serve_control<W: World, E: Effects>(&mut self, world: &mut W, runner: &mut Runner<E>) {
        let requests = world.take_requests();
        if requests.is_empty() {
            return;
        }
        let replies = requests
            .into_iter()
            .map(|request| serve_one(request, runner))
            .collect();
        world.answer(replies);
    }
}

fn tick_ms<E: Effects>(runner: &Runner<E>) -> Millis {
    runner.machine().tick_interval().as_millis() as Millis
}

fn serve_one<E: Effects>(request: Request, runner: &mut Runner<E>) -> Reply {
    match request {
        Request::Arm => {
            let was = runner.phase();
            runner.dispatch(Event::Arm);
            let detail = if was == Phase::Disarmed {
                "armed; one shot - it runs until the link returns, a disarm or a reboot".to_string()
            } else {
                format!("already {}; nothing to arm", was.as_str())
            };
            Reply::new(true, runner.phase().as_str(), detail)
        }
        Request::Disarm => {
            let was = runner.state();
            runner.dispatch(Event::Disarm);
            let phase = runner.phase().as_str();

            if let Some(err) = runner.health() {
                return Reply::new(
                    false,
                    phase,
                    format!("disarmed, but the wake alarm may still be set: {err}"),
                );
            }

            let detail = if was == State::Disarmed {
                "already disarmed; nothing was watching".to_string()
            } else if runner.machine().suspend_pending() {
                "disarmed, and nothing further will be done. A suspend was already \
                 handed to logind and cannot be recalled, so the machine may still \
                 go down once; its wake alarm was left set on purpose and will bring \
                 it back"
                    .to_string()
            } else {
                "disarmed; wake alarm cleared".to_string()
            };
            Reply::new(true, phase, detail)
        }
        Request::Status => {
            let detail = describe(runner);
            Reply::new(true, runner.phase().as_str(), detail)
        }
    }
}

fn describe<E: Effects>(runner: &Runner<E>) -> String {
    let now = runner.effects().now();
    let secs = |from: Millis| now.saturating_sub(from) / 1_000;

    let mut detail = match runner.state() {
        State::Disarmed => "idle; run `outage arm` to watch for an outage".to_string(),
        State::Armed { offline_since } => {
            let link = match offline_since {
                None => "the link is up".to_string(),
                Some(since) => format!("offline for {}s", secs(since)),
            };
            match runner.machine().blocked().describe() {
                Some(why) => format!("watching, but entry is blocked: {why} ({link})"),
                None => format!("watching; {link}"),
            }
        }
        State::Terminating { attempts, .. } => {
            format!("tearing the user session down (attempt {attempts} of {MAX_TEARDOWN_ATTEMPTS})")
        }
        State::Probing { until } => format!(
            "awake, checking the link for another {}s",
            until.saturating_sub(now) / 1_000
        ),
        State::Asleep => "sleeping between link checks".to_string(),
        State::Failed { failure, since } => format!(
            "degraded {}s ago: {}. Staying awake and not claiming otherwise; \
             the link returning still ends the protocol",
            secs(since),
            failure.as_str()
        ),
    };

    if runner.machine().suspend_pending() && !matches!(runner.state(), State::Asleep) {
        detail.push_str("; a suspend is still outstanding, so the wake alarm is being kept");
    }

    if let Some(err) = runner.health() {
        detail.push_str(&format!("; last failure: {err}"));
    }
    detail
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::io;
    use std::rc::Rc;

    use crate::machine::{Machine, Thresholds, MAX_SLEEP_FAILURES};

    const SEC: Millis = 1_000;
    const MIN: Millis = 60 * SEC;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Did {
        Terminated,
        ArmedWake,
        CancelledWake,
        Suspended,
    }

    #[derive(Clone)]
    struct Clock(Rc<RefCell<(Millis, Millis)>>);

    impl Clock {
        fn new(now: Millis) -> Self {
            Self(Rc::new(RefCell::new((now, now))))
        }
        fn now(&self) -> Millis {
            self.0.borrow().0
        }
        fn awake(&self) -> Millis {
            self.0.borrow().1
        }
        fn advance(&self, by: Millis) {
            let mut t = self.0.borrow_mut();
            t.0 += by;
            t.1 += by;
        }
        fn sleep(&self, by: Millis) {
            self.0.borrow_mut().0 += by;
        }
    }

    #[derive(Clone)]
    struct Recorder {
        clock: Clock,
        did: Rc<RefCell<Vec<Did>>>,
        terminate_fails: Rc<Cell<bool>>,
    }

    impl Recorder {
        fn drain(&self) -> Vec<Did> {
            std::mem::take(&mut self.did.borrow_mut())
        }

        fn refuse_teardowns(&self) {
            self.terminate_fails.set(true);
        }
    }

    impl Effects for Recorder {
        fn now(&self) -> Millis {
            self.clock.now()
        }
        fn terminate_user_session(&mut self, _grace: Duration) -> io::Result<()> {
            self.did.borrow_mut().push(Did::Terminated);
            if self.terminate_fails.get() {
                return Err(io::Error::other("logind refused"));
            }
            Ok(())
        }
        fn arm_wake(&mut self, _after: Duration) -> io::Result<()> {
            self.did.borrow_mut().push(Did::ArmedWake);
            Ok(())
        }
        fn cancel_wake(&mut self) -> io::Result<()> {
            self.did.borrow_mut().push(Did::CancelledWake);
            Ok(())
        }
        fn suspend(&mut self) -> io::Result<()> {
            self.did.borrow_mut().push(Did::Suspended);
            Ok(())
        }
    }

    struct Fake {
        clock: Clock,
        drains: VecDeque<Vec<Request>>,
        replies: Vec<Reply>,
        resume: Option<bool>,
        idle_since: Option<Millis>,
        inputs: VecDeque<Option<Millis>>,
        power: Power,
        probe_inflight: bool,
        probes_started: usize,
        result: Option<(Millis, bool)>,
    }

    impl Fake {
        fn new(clock: Clock) -> Self {
            Self {
                clock,
                drains: VecDeque::new(),
                replies: Vec::new(),
                resume: None,
                idle_since: Some(0),
                inputs: VecDeque::new(),
                power: Power::Battery,
                probe_inflight: false,
                probes_started: 0,
                result: None,
            }
        }

        fn deliver_on_drain(&mut self, n: usize, request: Request) {
            while self.drains.len() < n {
                self.drains.push_back(Vec::new());
            }
            self.drains[n - 1].push(request);
        }

        fn deliver_input_on_refresh(&mut self, n: usize, at: Millis) {
            while self.inputs.len() < n {
                self.inputs.push_back(None);
            }
            self.inputs[n - 1] = Some(at);
        }

        fn finish_probe(&mut self, online: bool) {
            assert!(self.probe_inflight, "no probe was asked for");
            self.probe_inflight = false;
            self.result = Some((self.clock.now(), online));
        }
    }

    impl World for Fake {
        fn take_requests(&mut self) -> Vec<Request> {
            self.drains.pop_front().unwrap_or_default()
        }
        fn answer(&mut self, replies: Vec<Reply>) {
            self.replies.extend(replies);
        }
        fn take_resume(&mut self) -> Option<bool> {
            self.resume.take()
        }
        fn refresh(&mut self) {
            if let Some(Some(at)) = self.inputs.pop_front() {
                self.idle_since = Some(at);
            }
        }
        fn idle_since(&self) -> Option<Millis> {
            self.idle_since
        }
        fn power(&mut self) -> Power {
            self.power
        }
        fn connectivity(&mut self) -> Option<(Millis, bool)> {
            self.result
        }
        fn request_probe(&mut self) {
            if !self.probe_inflight {
                self.probe_inflight = true;
                self.probes_started += 1;
            }
        }
        fn probe_inflight(&self) -> bool {
            self.probe_inflight
        }
        fn rescan_input(&mut self) {}
        fn awake_ms(&self) -> Millis {
            self.clock.awake()
        }
    }

    struct Rig {
        clock: Clock,
        world: Fake,
        runner: Runner<Recorder>,
        driver: Driver,
        log: Recorder,
    }

    impl Rig {
        fn new() -> Self {
            let clock = Clock::new(0);
            let log = Recorder {
                clock: clock.clone(),
                did: Rc::new(RefCell::new(Vec::new())),
                terminate_fails: Rc::new(Cell::new(false)),
            };
            Self {
                world: Fake::new(clock.clone()),
                runner: Runner::new(Machine::new(Thresholds::default()), log.clone()),
                driver: Driver::new(0, Duration::from_secs(6)),
                log,
                clock,
            }
        }

        fn step(&mut self) -> Millis {
            self.driver.step(&mut self.world, &mut self.runner)
        }

        fn settle(&mut self, online: bool) {
            for _ in 0..8 {
                self.step();
                if !self.world.probe_inflight {
                    return;
                }
                self.world.finish_probe(online);
            }
            panic!("the driver never stopped asking for a verdict");
        }

        fn arm(&mut self) {
            self.runner.dispatch(Event::Arm);
        }

        fn doze(&mut self) {
            self.arm();
            self.settle(false);

            self.clock.advance(6 * MIN);
            self.settle(false);
            assert_eq!(self.log.drain(), vec![Did::Terminated]);

            self.clock.advance(2 * MIN);
            self.settle(false);
            assert_eq!(self.runner.state(), State::Asleep);
            assert_eq!(self.log.drain(), vec![Did::ArmedWake, Did::Suspended]);
        }
    }

    #[test]
    fn a_tick_waits_for_a_fresh_verdict_instead_of_acting_on_none() {
        let mut rig = Rig::new();
        rig.arm();
        let wait = rig.step();
        assert_eq!(wait, PROBE_WAIT_MS, "it must come back promptly, not tick");
        assert_eq!(rig.world.probes_started, 1);
        assert!(rig.log.drain().is_empty());
    }

    #[test]
    fn a_stale_verdict_is_refused_and_re_probed() {
        let mut rig = Rig::new();
        rig.arm();
        rig.settle(false);

        rig.clock.advance(20 * SEC);
        rig.world.result = Some((0, false));
        let before = rig.world.probes_started;
        rig.step();
        assert_eq!(
            rig.world.probes_started,
            before + 1,
            "a verdict older than the limit must be re-measured, not reused"
        );
    }

    #[test]
    fn a_disarm_arriving_during_a_slow_probe_prevents_the_entry_it_raced() {
        let mut rig = Rig::new();
        rig.arm();

        rig.settle(false);
        rig.clock.advance(6 * MIN);
        rig.world.result = None;

        assert_eq!(rig.step(), PROBE_WAIT_MS);
        assert!(rig.world.probe_inflight);

        rig.world.finish_probe(false);
        rig.world.deliver_on_drain(2, Request::Disarm);
        rig.step();

        assert_eq!(
            rig.log.drain(),
            vec![Did::CancelledWake],
            "the disarm must be the only thing that happened"
        );
        assert_eq!(rig.runner.phase(), Phase::Disarmed);

        for _ in 0..10 {
            rig.clock.advance(MIN);
            rig.settle(false);
        }
        assert!(rig.log.drain().is_empty());
    }

    #[test]
    fn input_arriving_during_a_slow_probe_prevents_the_entry_it_raced() {
        let mut rig = Rig::new();
        rig.arm();
        rig.settle(false);
        rig.clock.advance(6 * MIN);
        rig.world.result = None;

        assert_eq!(rig.step(), PROBE_WAIT_MS);
        assert!(rig.world.probe_inflight);

        rig.clock.advance(3 * SEC);
        rig.world.finish_probe(false);
        rig.world.deliver_input_on_refresh(2, rig.clock.now());
        rig.step();

        assert!(
            rig.log.drain().is_empty(),
            "a session must not be terminated on idle that was already stale"
        );
        assert_eq!(rig.runner.phase(), Phase::Armed);
    }

    #[test]
    fn a_disarm_during_a_slow_probe_also_cancels_a_pending_suspend() {
        let mut rig = Rig::new();
        rig.arm();
        rig.settle(false);
        rig.clock.advance(6 * MIN);
        rig.settle(false);
        assert_eq!(rig.log.drain(), vec![Did::Terminated]);

        rig.clock.advance(2 * MIN);
        rig.world.result = None;
        assert_eq!(rig.step(), PROBE_WAIT_MS);
        rig.world.finish_probe(false);
        rig.world.deliver_on_drain(2, Request::Disarm);
        rig.step();

        assert_eq!(rig.log.drain(), vec![Did::CancelledWake]);
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
        assert_ne!(rig.runner.state(), State::Asleep);
    }

    #[test]
    fn a_resume_is_decided_on_a_verdict_taken_after_the_wake() {
        let mut rig = Rig::new();
        rig.doze();

        rig.clock.sleep(10 * MIN);
        rig.world.resume = Some(true);
        assert!(rig.world.result.is_some());
        assert_eq!(rig.step(), PROBE_WAIT_MS, "it must re-measure the link");

        rig.world.finish_probe(true);
        rig.step();
        assert_eq!(rig.log.drain(), vec![Did::CancelledWake]);
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
    }

    #[test]
    fn status_reports_a_blocked_sensor_rather_than_looking_healthy() {
        let mut rig = Rig::new();
        rig.arm();
        rig.world.power = Power::Unknown;
        rig.world.idle_since = None;
        rig.settle(false);

        rig.world.deliver_on_drain(1, Request::Status);
        rig.step();
        let reply = rig.world.replies.pop().expect("a status reply");
        assert_eq!(reply.phase, "armed");
        assert!(
            reply.detail.contains("entry is blocked"),
            "status said: {}",
            reply.detail
        );
        assert!(reply.detail.contains("unreadable"), "{}", reply.detail);
    }

    #[test]
    fn a_teardown_that_keeps_failing_degrades_instead_of_sleeping() {
        let mut rig = Rig::new();
        rig.log.refuse_teardowns();
        rig.arm();
        rig.settle(false);

        rig.clock.advance(6 * MIN);
        rig.settle(false);
        assert_eq!(rig.log.drain(), vec![Did::Terminated]);
        assert!(
            matches!(rig.runner.state(), State::Terminating { .. }),
            "a teardown that did not report a cleared cgroup is not a teardown"
        );

        for _ in 1..MAX_TEARDOWN_ATTEMPTS {
            rig.clock.advance(20 * SEC);
            rig.settle(false);
            assert_eq!(rig.log.drain(), vec![Did::Terminated]);
        }
        rig.clock.advance(20 * SEC);
        rig.settle(false);
        assert_eq!(rig.runner.phase(), Phase::Failed);

        rig.world.deliver_on_drain(1, Request::Status);
        rig.step();
        let reply = rig.world.replies.pop().expect("a status reply");
        assert_eq!(reply.phase, "failed");
        assert!(
            reply.detail.contains("could not be verifiably terminated"),
            "status said: {}",
            reply.detail
        );

        for _ in 0..20 {
            rig.clock.advance(15 * MIN);
            rig.settle(false);
        }
        assert!(
            rig.log.drain().iter().all(|did| *did == Did::Terminated),
            "a protocol that could not clear the session must never suspend"
        );
        assert_eq!(rig.runner.phase(), Phase::Failed);
    }

    #[test]
    fn a_real_suspend_is_never_mistaken_for_a_refused_one() {
        let mut rig = Rig::new();
        rig.doze();

        rig.clock.sleep(10 * 60 * MIN);
        for _ in 0..5 {
            rig.step();
        }
        assert_eq!(rig.runner.state(), State::Asleep);
        assert!(rig.log.drain().is_empty());

        rig.world.resume = Some(true);
        rig.settle(true);
        assert_eq!(rig.log.drain(), vec![Did::CancelledWake]);
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
    }

    #[test]
    fn a_suspend_that_never_happens_is_caught_instead_of_ignoring_the_link() {
        let mut rig = Rig::new();
        rig.doze();

        rig.clock.advance(30 * SEC);
        rig.step();
        assert_eq!(rig.runner.state(), State::Asleep);

        rig.clock.advance(61 * SEC);
        rig.step();
        assert!(
            matches!(rig.runner.state(), State::Probing { .. }),
            "a machine that is demonstrably awake must go back to watching the \
             link, not sit in Asleep ignoring it for a full sleep interval"
        );
        assert!(
            rig.log.drain().is_empty(),
            "the alarm must survive: it is the only thing that can rescue a \
             suspend that lands after we gave up on it"
        );
    }

    #[test]
    fn an_alarm_kept_past_a_watchdog_still_brings_a_late_suspend_back() {
        let mut rig = Rig::new();
        rig.doze();

        rig.clock.advance(91 * SEC);
        rig.step();
        assert!(matches!(rig.runner.state(), State::Probing { .. }));

        rig.clock.sleep(10 * MIN);
        rig.world.resume = Some(true);
        rig.settle(true);
        assert_eq!(rig.log.drain(), vec![Did::CancelledWake]);
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
    }

    #[test]
    fn a_disarm_after_a_watchdog_still_leaves_the_alarm_set() {
        let mut rig = Rig::new();
        rig.doze();

        rig.clock.advance(91 * SEC);
        rig.step();
        assert!(matches!(rig.runner.state(), State::Probing { .. }));

        rig.world.deliver_on_drain(1, Request::Disarm);
        rig.step();
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
        assert!(
            rig.log.drain().is_empty(),
            "the alarm outlives the protocol precisely because the suspend does"
        );

        let reply = rig.world.replies.pop().expect("a disarm reply");
        assert!(reply.ok);
        assert!(
            reply.detail.contains("may still go down once"),
            "the reply must not promise a wakefulness we cannot deliver: {}",
            reply.detail
        );

        rig.clock.sleep(10 * MIN);
        rig.world.resume = Some(true);
        rig.step();
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
        assert!(!rig.runner.machine().suspend_pending());
        assert_eq!(
            rig.log.drain(),
            vec![Did::CancelledWake],
            "an alarm nothing will ever replace must not outlive the suspend \
             it was bounding"
        );
    }

    #[test]
    fn a_disarm_and_a_manual_wake_in_one_pass_leave_no_alarm_behind() {
        let mut rig = Rig::new();
        rig.doze();

        rig.clock.sleep(2 * MIN);
        rig.world.resume = Some(false);
        rig.world.deliver_on_drain(1, Request::Disarm);
        rig.step();

        assert_eq!(rig.runner.phase(), Phase::Disarmed);
        assert!(!rig.runner.machine().suspend_pending());
        assert_eq!(
            rig.log.drain(),
            vec![Did::CancelledWake],
            "the suspend is over and the protocol is done; nothing may be left \
             that could wake the machine once the lid closes again"
        );

        let reply = rig.world.replies.pop().expect("a disarm reply");
        assert!(reply.ok);
        assert!(
            reply.detail.contains("wake alarm cleared"),
            "the resume was already observable, so the reply must not hedge: {}",
            reply.detail
        );

        for _ in 0..30 {
            rig.clock.advance(MIN);
            rig.settle(false);
        }
        assert!(rig.log.drain().is_empty());
    }

    #[test]
    fn status_owns_up_to_an_outstanding_suspend() {
        let mut rig = Rig::new();
        rig.doze();
        rig.clock.advance(91 * SEC);
        rig.step();

        rig.world.deliver_on_drain(1, Request::Status);
        rig.step();
        let reply = rig.world.replies.pop().expect("a status reply");
        assert_eq!(reply.phase, "active");
        assert!(
            reply.detail.contains("suspend is still outstanding"),
            "status said: {}",
            reply.detail
        );
    }

    #[test]
    fn suspends_that_keep_being_refused_park_the_protocol_awake() {
        let mut rig = Rig::new();
        rig.doze();

        for _ in 0..MAX_SLEEP_FAILURES - 1 {
            rig.clock.advance(91 * SEC);
            rig.step();
            assert!(matches!(rig.runner.state(), State::Probing { .. }));

            rig.clock.advance(61 * SEC);
            rig.settle(false);
            assert_eq!(rig.runner.state(), State::Asleep);
            assert_eq!(rig.log.drain(), vec![Did::ArmedWake, Did::Suspended]);
        }

        rig.clock.advance(91 * SEC);
        rig.step();
        assert_eq!(rig.runner.phase(), Phase::Failed);

        for _ in 0..20 {
            rig.clock.advance(15 * MIN);
            rig.settle(false);
        }
        assert!(rig.log.drain().is_empty());
        assert_eq!(rig.runner.phase(), Phase::Failed);

        rig.clock.advance(MIN);
        rig.settle(true);
        assert!(rig.log.drain().is_empty());
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
        assert!(rig.runner.machine().suspend_pending());
    }

    #[test]
    fn a_disarm_reaches_the_daemon_before_it_can_sleep_again() {
        let mut rig = Rig::new();
        rig.arm();
        rig.settle(false);
        rig.clock.advance(6 * MIN);

        rig.settle(false);
        assert_eq!(rig.log.drain(), vec![Did::Terminated]);
        assert!(matches!(rig.runner.state(), State::Probing { .. }));

        rig.clock.advance(20 * SEC);
        rig.world.deliver_on_drain(1, Request::Disarm);
        rig.step();
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
        assert_eq!(rig.log.drain(), vec![Did::CancelledWake]);

        for _ in 0..30 {
            rig.clock.advance(MIN);
            rig.settle(false);
        }
        assert!(rig.log.drain().is_empty());
    }

    #[test]
    fn arming_promises_a_protocol_that_outlives_its_own_entry() {
        let mut rig = Rig::new();
        rig.world.deliver_on_drain(1, Request::Arm);
        rig.settle(false);
        let reply = rig.world.replies.pop().expect("an arm reply");
        assert_eq!(reply.phase, "armed");

        rig.clock.advance(6 * MIN);
        rig.settle(false);
        assert_eq!(rig.log.drain(), vec![Did::Terminated]);
        assert_eq!(
            rig.runner.phase(),
            Phase::Active,
            "arming survives entry: {}",
            reply.detail
        );
        assert!(!reply.detail.contains("entry"), "{}", reply.detail);

        rig.clock.advance(10 * SEC);
        rig.settle(true);
        assert_eq!(rig.runner.phase(), Phase::Disarmed);
    }

    #[test]
    fn a_disarmed_controller_never_probes() {
        let mut rig = Rig::new();
        for _ in 0..20 {
            rig.clock.advance(MIN);
            rig.step();
        }
        assert_eq!(
            rig.world.probes_started, 0,
            "a disarmed controller has nothing to measure"
        );
        assert!(rig.log.drain().is_empty());
    }
}

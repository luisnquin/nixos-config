use std::sync::Arc;

use ratatui::widgets::ListState;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use crate::actions::{self, Sink};
use crate::connect::{self, Reporter, Step};
use crate::discover::survey;
use crate::model::{Platform, Reach, View};
use crate::registry::Registry;

pub type Shared = Arc<Mutex<Registry>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Try,
    Done,
    Fail,
    Note,
}

pub struct LogLine {
    pub level: Level,
    pub text: String,
}

pub enum Msg {
    Views(Vec<View>),
    Step(Step),
    Finished(Result<String, String>),
    Exec(std::process::Command),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Prompt {
    Logs,
    PairCode,
}

impl Prompt {
    pub fn label(self) -> &'static str {
        match self {
            Prompt::Logs => "package / bundle id",
            Prompt::PairCode => "pairing code",
        }
    }
}

pub enum Mode {
    Normal,
    Filter,
    Prompt(Prompt),
    Help,
}

pub enum Outcome {
    Quit,
    Exec(std::process::Command),
}

/// A shot that has been asked for but has nothing to run against yet. The iPhone
/// tunnel drops often enough that the row itself disappears, so the request is
/// keyed on the device id and outlives the view it came from.
#[derive(Clone)]
pub struct Queued {
    pub id: String,
    pub label: String,
    pub tries: u8,
}

const MAX_SHOT_TRIES: u8 = 3;

pub struct App {
    pub reg: Shared,
    pub views: Vec<View>,
    pub visible: Vec<usize>,
    pub state: ListState,
    pub log: Vec<LogLine>,
    pub mode: Mode,
    pub filter: String,
    pub input: String,
    pub busy: Option<String>,
    pub progress: Option<(usize, usize)>,
    pub current: Option<String>,
    pub quit: Option<Outcome>,
    pub queue: Vec<Queued>,
    shot: Option<Queued>,
    tx: UnboundedSender<Msg>,
}

impl App {
    pub fn new(reg: Shared, current: Option<String>, tx: UnboundedSender<Msg>) -> Self {
        Self {
            reg,
            views: Vec::new(),
            visible: Vec::new(),
            state: ListState::default(),
            log: Vec::new(),
            mode: Mode::Normal,
            filter: String::new(),
            input: String::new(),
            busy: None,
            progress: None,
            current,
            quit: None,
            queue: Vec::new(),
            shot: None,
            tx,
        }
    }

    pub fn selected(&self) -> Option<&View> {
        let i = self.state.selected()?;

        self.views.get(*self.visible.get(i)?)
    }

    fn picked(&self) -> Option<(String, String)> {
        self.selected()
            .map(|v| (v.device.id.clone(), v.device.label.clone()))
    }

    pub fn push_log(&mut self, level: Level, text: impl Into<String>) {
        self.log.push(LogLine {
            level,
            text: text.into(),
        });

        // the pane only ever shows a tail; an unbounded log is a slow leak in a
        // process that is meant to stay open all day.
        if self.log.len() > 500 {
            self.log.drain(..self.log.len() - 500);
        }
    }

    pub fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Views(views) => {
                let keep = self.selected().map(|v| v.device.id.clone());

                self.views = views;
                self.refilter();

                if let Some(id) = keep {
                    if let Some(pos) = self
                        .visible
                        .iter()
                        .position(|i| self.views[*i].device.id == id)
                    {
                        self.state.select(Some(pos));
                    }
                }

                self.drain_queue();
            }
            Msg::Step(step) => match step {
                Step::Try(t) => self.push_log(Level::Try, t),
                Step::Done(t) => self.push_log(Level::Done, t),
                Step::Fail(t) => self.push_log(Level::Fail, t),
                Step::Note(t) => self.push_log(Level::Note, t),
                Step::Progress { done, total } => self.progress = Some((done, total)),
            },
            Msg::Finished(res) => {
                self.busy = None;
                self.progress = None;

                let shot = self.shot.take();

                match res {
                    Ok(text) if !text.is_empty() => self.push_log(Level::Done, text),
                    Ok(_) => {}
                    Err(e) => {
                        self.push_log(Level::Fail, e);

                        if let Some(shot) = shot {
                            self.requeue(shot);
                        }
                    }
                }

                self.refresh();
            }
            Msg::Exec(cmd) => self.quit = Some(Outcome::Exec(cmd)),
        }
    }

    pub fn refilter(&mut self) {
        self.visible = self
            .views
            .iter()
            .enumerate()
            .filter(|(_, v)| self.filter.is_empty() || v.device.matches(&self.filter))
            .map(|(i, _)| i)
            .collect();

        if self.visible.is_empty() {
            self.state.select(None);
        } else {
            let sel = self.state.selected().unwrap_or(0).min(self.visible.len() - 1);
            self.state.select(Some(sel));
        }
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }

        let len = self.visible.len() as isize;
        let cur = self.state.selected().unwrap_or(0) as isize;

        self.state.select(Some((cur + delta).rem_euclid(len) as usize));
    }

    pub fn select_edge(&mut self, last: bool) {
        if self.visible.is_empty() {
            return;
        }

        self.state
            .select(Some(if last { self.visible.len() - 1 } else { 0 }));
    }

    pub fn refresh(&mut self) {
        if self.busy.is_some() {
            // a survey and a running action would fight over the registry lock
            return;
        }

        let reg = self.reg.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let views = {
                let mut guard = reg.lock().await;
                let views = survey(&mut guard).await;
                let _ = guard.save();
                views
            };

            let _ = tx.send(Msg::Views(views));
        });
    }

    fn reporter(&self) -> (Reporter, tokio::task::JoinHandle<()>) {
        let (step_tx, mut step_rx) = tokio::sync::mpsc::unbounded_channel();
        let tx = self.tx.clone();

        let pump = tokio::spawn(async move {
            while let Some(step) = step_rx.recv().await {
                if tx.send(Msg::Step(step)).is_err() {
                    break;
                }
            }
        });

        (Reporter::new(step_tx), pump)
    }

    /// Runs one registry-mutating action in the background. Only one at a time:
    /// the lock is held for its whole duration, and two connects racing on the
    /// same adb server produce nothing useful anyway.
    fn spawn<F, Fut>(&mut self, label: impl Into<String>, f: F)
    where
        F: FnOnce(Shared, Reporter) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
    {
        if self.busy.is_some() {
            self.push_log(Level::Fail, "busy; wait for the current step to finish");
            return;
        }

        let label = label.into();

        self.push_log(Level::Try, label.clone());
        self.busy = Some(label);

        let (rep, pump) = self.reporter();
        let reg = self.reg.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let res = f(reg, rep).await;

            pump.abort();

            let _ = tx.send(Msg::Finished(res.map_err(|e| e.to_string())));
        });
    }

    pub fn connect_selected(&mut self) {
        let Some(view) = self.selected() else { return };

        let device = view.device.clone();
        let label = device.label.clone();

        if !device.platform.is_adb() {
            self.push_log(
                Level::Note,
                format!("{label} is already reachable through {}", crate::ios::HOST),
            );

            return;
        }

        self.spawn(format!("connect {label}"), move |reg, rep| async move {
            let mut guard = reg.lock().await;

            connect::connect(&mut guard, &device, &connect::Opts::default(), &rep).await
        });
    }

    pub fn disconnect_selected(&mut self) {
        let Some(view) = self.selected() else { return };

        let label = view.device.label.clone();
        let serial = view.reach.serial().map(str::to_string);

        let Some(serial) = serial else {
            self.push_log(Level::Fail, format!("{label} is not attached"));
            return;
        };

        if !serial.contains(':') {
            self.push_log(Level::Fail, "USB transports cannot be disconnected");
            return;
        }

        self.spawn(format!("disconnect {label}"), move |_reg, _rep| async move {
            crate::adb::disconnect(&serial).await?;

            anyhow::Ok(format!("disconnected {serial}"))
        });
    }

    /// Queues a shot for the selected device, or drops the one already queued.
    /// Nothing here needs the device to be reachable: that is the whole point.
    pub fn shot_selected(&mut self) {
        let Some((id, label)) = self.picked() else {
            return;
        };

        if let Some(pos) = self.queue.iter().position(|q| q.id == id) {
            self.queue.remove(pos);
            self.push_log(Level::Note, format!("dropped the queued shot for {label}"));

            return;
        }

        self.queue.push(Queued {
            id: id.clone(),
            label: label.clone(),
            tries: 0,
        });

        self.drain_queue();

        if self.queue.iter().any(|q| q.id == id) {
            self.push_log(Level::Note, format!("queued a shot for {label}"));
        }
    }

    fn ready_to_shoot(&self, id: &str) -> bool {
        self.views
            .iter()
            .any(|v| v.device.id == id && v.can_shoot())
    }

    /// Fires the first queued shot whose device is back. Called on every survey,
    /// so a device that reappears is captured within one tick of showing up.
    fn drain_queue(&mut self) {
        if self.busy.is_some() {
            return;
        }

        let Some(pos) = self.queue.iter().position(|q| self.ready_to_shoot(&q.id)) else {
            return;
        };

        let mut shot = self.queue.remove(pos);
        shot.tries += 1;

        let Some(device) = self
            .views
            .iter()
            .find(|v| v.device.id == shot.id)
            .map(|v| v.device.clone())
        else {
            return;
        };

        let label = device.label.clone();

        self.shot = Some(shot);

        self.spawn(format!("shot {label}"), move |_reg, rep| async move {
            actions::screenshot(&device, &Sink::Clipboard, &rep).await
        });
    }

    /// A capture that died because the device went away is the case the queue
    /// exists for, so it goes back in line. Bounded: one that is still listed and
    /// still failing (locked, asleep) would otherwise retry forever.
    fn requeue(&mut self, shot: Queued) {
        if shot.tries >= MAX_SHOT_TRIES {
            self.push_log(
                Level::Fail,
                format!("gave up on {} after {} tries", shot.label, shot.tries),
            );

            return;
        }

        self.push_log(Level::Note, format!("re-queued the shot for {}", shot.label));
        self.queue.push(shot);
    }

    /// The queue outlives the rows it was built from, so it needs a line of its
    /// own; a device waiting on the tunnel has nothing left in the list.
    pub fn queued_label(&self) -> Option<String> {
        if self.queue.is_empty() {
            return None;
        }

        Some(
            self.queue
                .iter()
                .map(|q| q.label.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )
    }

    pub fn mirror_selected(&mut self) {
        let Some(view) = self.selected() else { return };

        let device = view.device.clone();

        self.spawn(
            format!("mirror {}", device.label),
            move |_reg, _rep| async move { actions::mirror(&device).await },
        );
    }

    pub fn pin_selected(&mut self) {
        let Some(view) = self.selected() else { return };

        let device = view.device.clone();

        if !device.platform.is_adb() {
            self.push_log(Level::Note, format!("{} has no adb port to pin", device.label));
            return;
        }

        if device.platform == Platform::Emulator {
            self.push_log(Level::Fail, "emulators already listen on a fixed port");
            return;
        }

        self.spawn(format!("pin {}", device.label), move |reg, rep| async move {
            let mut guard = reg.lock().await;

            connect::pin(&mut guard, &device, 5555, &rep).await?;

            anyhow::Ok(String::new())
        });
    }

    pub fn forget_selected(&mut self) {
        let Some((id, label)) = self.picked() else {
            return;
        };

        self.spawn(format!("forget {label}"), move |reg, _rep| async move {
            let mut guard = reg.lock().await;

            let removed = guard.remove(&id);
            guard.save()?;

            anyhow::Ok(if removed {
                format!("forgot {label}")
            } else {
                format!("{label} was not in the registry")
            })
        });
    }

    pub fn use_selected(&mut self) {
        let Some((id, label)) = self.picked() else {
            return;
        };

        self.current = Some(id.clone());

        self.spawn(format!("use {label}"), move |reg, _rep| async move {
            let mut guard = reg.lock().await;

            guard.current = Some(id);
            guard.save()?;

            anyhow::Ok(format!("default target is {label}"))
        });
    }

    pub fn submit_prompt(&mut self, prompt: Prompt) {
        let value = std::mem::take(&mut self.input);

        self.mode = Mode::Normal;

        if value.is_empty() {
            return;
        }

        match prompt {
            Prompt::Logs => {
                let Some(view) = self.selected() else { return };

                let device = view.device.clone();
                let tx = self.tx.clone();

                self.push_log(Level::Try, format!("logs {} {value}", device.label));

                // logcat and oslog own the terminal, so the TUI has to be torn
                // down first; the command goes back to main to be run there.
                tokio::spawn(async move {
                    let msg = match actions::logs_command(&device, &value).await {
                        Ok(cmd) => Msg::Exec(cmd),
                        Err(e) => Msg::Finished(Err(e.to_string())),
                    };

                    let _ = tx.send(msg);
                });
            }
            Prompt::PairCode => {
                self.spawn("pair", move |reg, rep| async move {
                    let addr = connect::pair(None, &value, &rep).await?;

                    let guard = reg.lock().await;
                    guard.save()?;

                    anyhow::Ok(format!("paired {addr}; connect it now"))
                });
            }
        }
    }

    pub fn quit(&mut self) {
        self.quit = Some(Outcome::Quit);
    }

    pub fn header(&self) -> String {
        let attached = self.views.iter().filter(|v| v.reach.is_attached()).count();
        let online = self
            .views
            .iter()
            .filter(|v| v.reach == Reach::Online)
            .count();

        format!(
            "{} device(s)   {attached} attached   {online} online",
            self.views.len()
        )
    }

    /// Footer hints for the selected row. Offering `connect` or `pin` on a
    /// device that has no adb transport at all only invites the keystroke.
    pub fn hints(&self) -> &'static [&'static str] {
        const IOS: &[&str] = &["s shot", "l logs", "u use", "/ filter", "? help", "q quit"];
        const EMULATOR: &[&str] = &[
            "enter connect",
            "s shot",
            "m mirror",
            "l logs",
            "/ filter",
            "? help",
            "q quit",
        ];
        const ANDROID: &[&str] = &[
            "enter connect",
            "s shot",
            "m mirror",
            "p pin",
            "/ filter",
            "? help",
            "q quit",
        ];

        match self.selected().map(|v| v.device.platform) {
            Some(Platform::Ios) => IOS,
            Some(Platform::Emulator) => EMULATOR,
            _ => ANDROID,
        }
    }

    pub fn current_label(&self) -> Option<String> {
        let id = self.current.as_ref()?;

        Some(
            self.views
                .iter()
                .find(|v| &v.device.id == id)
                .map(|v| v.device.label.clone())
                .unwrap_or_else(|| id.clone()),
        )
    }
}

/// label, model, reachability, endpoint + last-connected — the four columns the
/// list draws.
pub fn row_fields(view: &View) -> (String, String, String, String) {
    let d = &view.device;

    let endpoint = d
        .ranked_endpoints()
        .first()
        .map(|e| {
            let pin = e.pin.as_str();

            if pin.is_empty() {
                e.addr()
            } else {
                format!("{} {pin}", e.addr())
            }
        })
        .unwrap_or_default();

    let last = d
        .last_connected
        .map(crate::model::ago)
        .unwrap_or_else(|| "never".into());

    let detail = match d.platform {
        // there is no endpoint and no connect history to report: the tunnel on
        // the mac either has the device or it does not.
        Platform::Ios => format!("via {}", crate::ios::HOST),
        _ if endpoint.is_empty() => last,
        _ => format!("{endpoint}  {last}"),
    };

    (d.label.clone(), d.model.clone(), view.reach.label(), detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Device;

    fn view(id: &str, platform: Platform, reach: Reach) -> View {
        View {
            device: Device::new(id, id, platform),
            reach,
        }
    }

    fn app_with(views: Vec<View>) -> App {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let mut app = App::new(Arc::new(Mutex::new(Registry::default())), None, tx);

        app.views = views;
        app.refilter();
        app
    }

    #[test]
    fn a_tailnet_answer_is_not_a_transport() {
        let app = app_with(vec![
            view("phone", Platform::Android, Reach::Online),
            view("udid", Platform::Ios, Reach::Online),
        ]);

        assert!(!app.ready_to_shoot("phone"));
        assert!(app.ready_to_shoot("udid"));
    }

    #[test]
    fn an_unreachable_device_queues_the_shot() {
        let mut app = app_with(vec![view("phone", Platform::Android, Reach::Online)]);

        app.shot_selected();

        assert_eq!(app.queue.len(), 1);
        assert!(app.busy.is_none(), "ran a capture with no transport to run it on");
    }

    #[test]
    fn shooting_a_queued_device_drops_it() {
        let mut app = app_with(vec![view("phone", Platform::Android, Reach::Online)]);

        app.shot_selected();
        app.shot_selected();

        assert!(app.queue.is_empty());
    }

    #[test]
    fn a_queued_shot_outlives_its_row() {
        let mut app = app_with(vec![view("udid", Platform::Ios, Reach::Known)]);

        app.shot_selected();
        app.on_msg(Msg::Views(Vec::new()));

        assert_eq!(app.queue.len(), 1);
        assert_eq!(app.queued_label().as_deref(), Some("udid"));
    }

    #[test]
    fn a_failed_shot_goes_back_in_line_until_the_limit() {
        let mut app = app_with(Vec::new());

        app.requeue(Queued {
            id: "udid".into(),
            label: "udid".into(),
            tries: 1,
        });

        assert_eq!(app.queue.len(), 1);

        app.queue.clear();
        app.requeue(Queued {
            id: "udid".into(),
            label: "udid".into(),
            tries: MAX_SHOT_TRIES,
        });

        assert!(app.queue.is_empty(), "retried a device that keeps failing while present");
    }
}

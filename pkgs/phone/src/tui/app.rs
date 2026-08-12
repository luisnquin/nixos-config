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

                match res {
                    Ok(text) if !text.is_empty() => self.push_log(Level::Done, text),
                    Ok(_) => {}
                    Err(e) => self.push_log(Level::Fail, e),
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

    pub fn shot_selected(&mut self) {
        let Some(view) = self.selected() else { return };

        let device = view.device.clone();

        self.spawn(
            format!("shot {}", device.label),
            move |_reg, rep| async move { actions::screenshot(&device, &Sink::Clipboard, &rep).await },
        );
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

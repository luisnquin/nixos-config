pub mod dsp;
pub mod engine;
#[cfg(feature = "audio")]
mod sink;

use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crate::carousel::Carousel;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cue {
    Key,
    Erase,
    Nav,
    Submit,
    Scan(bool),
    Fail,
    Grant,
    Cancel,
    Fade,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub device: Option<String>,
    pub volume: f32,
    pub tick_ms: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            device: None,
            volume: 0.4,
            tick_ms: 120,
        }
    }
}

#[cfg_attr(not(feature = "audio"), allow(dead_code))]
enum Msg {
    Tick(u64),
    Cue(Cue),
}

pub struct Sound {
    tx: Sender<Msg>,
    done: Receiver<()>,
}

impl Sound {
    #[cfg(feature = "audio")]
    pub fn open(options: Options, carousel: Carousel) -> Option<Sound> {
        let sink = sink::open(options.device.as_deref())?;
        let (tx, rx) = std::sync::mpsc::channel();
        let (done_tx, done) = std::sync::mpsc::channel();
        let engine =
            engine::Engine::new(sink.rate as f32, options.tick_ms, carousel, options.volume);
        std::thread::Builder::new()
            .name("ttycanvas-audio".into())
            .spawn(move || {
                sink::run(sink, engine, rx);
                drop(done_tx);
            })
            .ok()?;
        Some(Sound { tx, done })
    }

    #[cfg(not(feature = "audio"))]
    pub fn open(_options: Options, _carousel: Carousel) -> Option<Sound> {
        None
    }

    pub fn tick(&self, tick: u64) {
        let _ = self.tx.send(Msg::Tick(tick));
    }

    pub fn cue(&self, cue: Cue) {
        let _ = self.tx.send(Msg::Cue(cue));
    }

    pub fn finish(self) {
        let _ = self.tx.send(Msg::Cue(Cue::Fade));
        let _ = self.done.recv_timeout(Duration::from_millis(1200));
    }
}

#[cfg_attr(not(feature = "audio"), allow(dead_code))]
fn drain(rx: &Receiver<Msg>, engine: &mut engine::Engine) {
    loop {
        match rx.try_recv() {
            Ok(Msg::Tick(t)) => engine.sync(t),
            Ok(Msg::Cue(c)) => engine.cue(c),
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                engine.cue(Cue::Fade);
                return;
            }
        }
    }
}

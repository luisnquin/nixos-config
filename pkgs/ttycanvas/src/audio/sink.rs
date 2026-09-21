use std::sync::mpsc::Receiver;

use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::{Direction, ValueOr};

use super::engine::Engine;
use super::{Msg, drain};

pub struct Sink {
    pcm: PCM,
    pub rate: u32,
    period: usize,
}

/// The configured device, else ALSA's default, else every card's `plughw`
/// in turn. A greeter user without a sound server still finds the hardware.
pub fn open(device: Option<&str>) -> Option<Sink> {
    let mut names = Vec::new();
    match device {
        Some(d) => names.push(d.to_string()),
        None => {
            names.push("default".to_string());
            for card in alsa::card::Iter::new().flatten() {
                names.push(format!("plughw:{}", card.get_index()));
            }
        }
    }
    names.iter().find_map(|name| try_open(name).ok())
}

fn try_open(name: &str) -> alsa::Result<Sink> {
    let pcm = PCM::new(name, Direction::Playback, false)?;
    {
        let hw = HwParams::any(&pcm)?;
        hw.set_channels(2)?;
        hw.set_rate_near(48_000, ValueOr::Nearest)?;
        hw.set_format(Format::s16())?;
        hw.set_access(Access::RWInterleaved)?;
        hw.set_period_size_near(512, ValueOr::Nearest)?;
        hw.set_buffer_size_near(2048)?;
        pcm.hw_params(&hw)?;
    }
    let (rate, period) = {
        let hw = pcm.hw_params_current()?;
        (hw.get_rate()?, hw.get_period_size()? as usize)
    };
    {
        let sw = pcm.sw_params_current()?;
        sw.set_start_threshold((period * 2) as i64)?;
        pcm.sw_params(&sw)?;
    }
    Ok(Sink { pcm, rate, period })
}

impl Sink {
    fn write(&self, buf: &[i16]) -> bool {
        let Ok(io) = self.pcm.io_i16() else {
            return false;
        };
        let mut offset = 0;
        while offset < buf.len() {
            match io.writei(&buf[offset..]) {
                Ok(frames) => offset += frames * 2,
                Err(e) => {
                    if self.pcm.try_recover(e, true).is_err() {
                        return false;
                    }
                }
            }
        }
        true
    }
}

pub fn run(sink: Sink, mut engine: Engine, rx: Receiver<Msg>) {
    let mut samples = vec![0.0f32; sink.period * 2];
    let mut pcm = vec![0i16; sink.period * 2];
    loop {
        drain(&rx, &mut engine);
        engine.render(&mut samples);
        for (dst, src) in pcm.iter_mut().zip(&samples) {
            *dst = (src.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        }
        if !sink.write(&pcm) {
            return;
        }
        if engine.finished() {
            let _ = sink.pcm.drain();
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Cue;
    use crate::carousel::{Carousel, SLOT};

    #[test]
    #[ignore = "plays two seconds on the host's sound card"]
    fn plays_on_the_host() {
        let sink = open(std::env::var("TTYCANVAS_DEVICE").ok().as_deref())
            .expect("no playback device opened");
        let (tx, rx) = std::sync::mpsc::channel();
        let mut engine = Engine::new(sink.rate as f32, 120, Carousel::new(7), 0.5);
        engine.sync(SLOT * 5 - 8);
        for cue in [Cue::Key, Cue::Key, Cue::Submit, Cue::Scan(true)] {
            tx.send(Msg::Cue(cue)).unwrap();
        }
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            tx.send(Msg::Cue(Cue::Grant)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(300));
            tx.send(Msg::Cue(Cue::Fade)).unwrap();
        });
        run(sink, engine, rx);
    }
}

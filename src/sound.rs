//! Piezo-style beeps: plain square waves, rendered once at startup.

use game_toolkit::prelude::*;

use crate::logic::Cue;

const RATE: u32 = 44_100;
const VOLUME: f32 = 0.22;

/// A square-wave tone of `freq` Hz for `secs`, with a short fade so it doesn't click.
fn tone(freq: f32, secs: f32) -> Vec<f32> {
    let n = (secs * RATE as f32) as usize;
    let fade = (RATE as f32 * 0.002) as usize;
    (0..n)
        .map(|i| {
            let phase = (i as f32 * freq / RATE as f32).fract();
            let s = if phase < 0.5 { VOLUME } else { -VOLUME };
            let edge = i.min(n - 1 - i);
            s * (edge as f32 / fade as f32).min(1.0)
        })
        .collect()
}

fn silence(secs: f32) -> Vec<f32> {
    vec![0.0; (secs * RATE as f32) as usize]
}

fn seq(parts: &[(f32, f32)]) -> Vec<f32> {
    parts
        .iter()
        .flat_map(|&(f, d)| if f > 0.0 { tone(f, d) } else { silence(d) })
        .collect()
}

pub struct Sounds {
    step: Vec<f32>,
    catch: Vec<f32>,
    miss: Vec<f32>,
    clear: Vec<f32>,
    game_over: Vec<f32>,
    alarm: Vec<f32>,
    pub muted: bool,
}

impl Sounds {
    pub fn new() -> Self {
        Self {
            step: tone(1600.0, 0.025),
            catch: tone(2400.0, 0.05),
            miss: seq(&[
                (900.0, 0.12),
                (0.0, 0.04),
                (700.0, 0.12),
                (0.0, 0.04),
                (500.0, 0.25),
            ]),
            clear: seq(&[
                (1500.0, 0.08),
                (2000.0, 0.08),
                (2500.0, 0.08),
                (3000.0, 0.16),
            ]),
            game_over: seq(&[
                (1200.0, 0.2),
                (0.0, 0.05),
                (1000.0, 0.2),
                (0.0, 0.05),
                (800.0, 0.2),
                (0.0, 0.05),
                (600.0, 0.5),
            ]),
            // A rattling telephone-bell burst.
            alarm: (0..10)
                .flat_map(|_| seq(&[(2000.0, 0.02), (0.0, 0.02)]))
                .collect(),
            muted: false,
        }
    }

    pub fn play(&self, audio: Option<&mut Audio>, cue: Cue) {
        let Some(audio) = audio else { return };
        if self.muted {
            return;
        }
        let buf = match cue {
            Cue::Step => &self.step,
            Cue::Catch => &self.catch,
            Cue::Miss => &self.miss,
            Cue::Clear => &self.clear,
            Cue::GameOver => &self.game_over,
            Cue::Alarm => &self.alarm,
        };
        // Fails until the browser has seen a user gesture; nothing to do but stay quiet.
        if let Err(e) = audio.play_samples(buf, RATE) {
            log::debug!("sound: {e}");
        }
    }
}

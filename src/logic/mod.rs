//! Device logic: game rounds, clock, alarm, and which LCD segments are lit.
//!
//! Pure Rust with no toolkit types, so it is fully unit-testable.

pub mod clock;
pub mod game;
pub mod speed;

use clock::{Clock, Shown};
use game::{FloorView, GAME_OVER_HALVES, POSITIONS, Round};

/// The four ramps, each with a big button and a wolf pose.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ramp {
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
}

impl Ramp {
    pub const ALL: [Ramp; 4] = [
        Ramp::TopLeft,
        Ramp::BottomLeft,
        Ramp::TopRight,
        Ramp::BottomRight,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn side(self) -> Side {
        match self {
            Ramp::TopLeft | Ramp::BottomLeft => Side::Left,
            Ramp::TopRight | Ramp::BottomRight => Side::Right,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Variant {
    A,
    B,
}

/// Sounds the piezo should make.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cue {
    Step,
    Catch,
    Miss,
    Clear,
    GameOver,
    Alarm,
}

/// A source of uniform `[0, 1)` randomness.
pub trait Random {
    fn next_f32(&mut self) -> f32;
}

/// Buttons pressed (edge-triggered) this update, plus whether Время is held.
#[derive(Copy, Clone, Debug, Default)]
pub struct Inputs {
    pub ramp: Option<Ramp>,
    pub game_a: bool,
    pub game_b: bool,
    pub time: bool,
    pub time_held: bool,
    pub alarm_set: bool,
    pub time_set: bool,
}

/// How long the high score is shown before a game starts.
pub const INTRO_TIME: f32 = 1.5;

#[derive(Clone, Debug)]
pub enum Mode {
    Clock,
    SetTime,
    SetAlarm,
    Intro { variant: Variant, t: f32 },
    Play,
}

pub struct Machine {
    pub mode: Mode,
    pub clock: Clock,
    /// The running game in `Play`, the attract demo otherwise.
    pub round: Round,
    pub hiscore: u32,
    /// Set when `hiscore` changed and should be persisted.
    pub hiscore_dirty: bool,
    blink: f32,
}

/// One 7-segment-style digit cell; `None` is blank.
pub type Digit = Option<u8>;

/// Everything the LCD shows this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub eggs: [[bool; POSITIONS]; 4],
    pub wolf: Option<Ramp>,
    pub hare: bool,
    /// Bell frame (0 or 1) while the alarm rings.
    pub bell: Option<u8>,
    pub left: FloorView,
    pub right: FloorView,
    pub misses: [bool; 3],
    /// Leading "1" of the hour or of a four-digit number.
    pub one: bool,
    pub digits: [Digit; 3],
    pub colon: bool,
    pub am: bool,
    pub pm: bool,
    pub alarm_icon: bool,
    pub game_a: bool,
    pub game_b: bool,
}

impl Machine {
    pub fn new(seconds_of_day: u32, hiscore: u32) -> Self {
        Self {
            mode: Mode::Clock,
            clock: Clock::new(seconds_of_day),
            round: Round::new(Variant::A, true),
            hiscore,
            hiscore_dirty: false,
            blink: 0.0,
        }
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.mode, Mode::Play) && !self.round.demo
    }

    pub fn update(&mut self, dt: f32, input: &Inputs, rng: &mut dyn Random) -> Vec<Cue> {
        let mut cues = Vec::new();
        self.blink = (self.blink + dt) % 1.0;
        self.clock.update(dt, &mut cues);

        if self.clock.is_ringing() && (input.time || input.ramp.is_some()) {
            self.clock.stop_ringing();
            return cues;
        }

        if input.game_a || input.game_b {
            let in_set_alarm = matches!(self.mode, Mode::SetAlarm);
            if in_set_alarm && input.game_a {
                self.clock.alarm_on = !self.clock.alarm_on;
            } else if !in_set_alarm {
                let variant = if input.game_a { Variant::A } else { Variant::B };
                self.mode = Mode::Intro { variant, t: 0.0 };
                return cues;
            }
        }

        match &mut self.mode {
            Mode::Clock => {
                if input.alarm_set {
                    self.mode = Mode::SetAlarm;
                } else if input.time_set {
                    self.mode = Mode::SetTime;
                }
                self.update_demo(dt, rng);
            }
            Mode::SetTime => {
                match input.ramp {
                    Some(Ramp::TopRight) => self.clock.set_hour(),
                    Some(Ramp::BottomRight) => self.clock.set_minute(),
                    Some(_) => self.clock.toggle_pm(),
                    None => {}
                }
                if input.time_set || input.time {
                    self.mode = Mode::Clock;
                }
            }
            Mode::SetAlarm => {
                match input.ramp {
                    Some(Ramp::TopRight) => self.clock.alarm_hour(),
                    Some(Ramp::BottomRight) => self.clock.alarm_minute(),
                    Some(_) => self.clock.alarm_toggle_pm(),
                    None => {}
                }
                if input.alarm_set || input.time {
                    self.mode = Mode::Clock;
                }
            }
            Mode::Intro { variant, t } => {
                *t += dt;
                if *t >= INTRO_TIME {
                    self.round = Round::new(*variant, false);
                    self.mode = Mode::Play;
                } else if input.time {
                    self.enter_clock();
                }
            }
            Mode::Play => {
                if input.time {
                    self.enter_clock();
                    return cues;
                }
                if let Some(r) = input.ramp {
                    self.round.move_wolf(r);
                }
                self.round.update(dt, rng, &mut cues);
                if self.round.total > self.hiscore {
                    self.hiscore = self.round.total;
                    self.hiscore_dirty = true;
                }
            }
        }
        cues
    }

    /// Advance only the clock, e.g. while the game is paused in the background.
    pub fn tick_clock(&mut self, dt: f32) {
        self.clock.update(dt, &mut Vec::new());
    }

    fn enter_clock(&mut self) {
        self.mode = Mode::Clock;
        self.round = Round::new(Variant::A, true);
    }

    fn update_demo(&mut self, dt: f32, rng: &mut dyn Random) {
        // The demo is silent.
        self.round.update(dt, rng, &mut Vec::new());
    }

    pub fn view(&self, input: &Inputs) -> View {
        let blink_on = self.blink < 0.5;
        let mut v = View::default();

        let show_round = matches!(self.mode, Mode::Clock | Mode::Play);
        if show_round {
            let r = &self.round;
            v.eggs = r.eggs;
            v.wolf = Some(r.wolf);
            v.hare = r.hare;
            v.left = r.floor(Side::Left);
            v.right = r.floor(Side::Right);
        } else {
            v.wolf = Some(Ramp::TopLeft);
        }

        if self.clock.is_ringing() {
            v.hare = true;
            v.bell = Some(((self.clock.ring_phase() * 4.0) as u32 % 2) as u8);
        }

        match &self.mode {
            Mode::Clock | Mode::SetTime | Mode::SetAlarm => {
                let alarm_view = matches!(self.mode, Mode::SetAlarm)
                    || (matches!(self.mode, Mode::Clock) && input.time_held);
                let shown = if alarm_view {
                    self.clock.alarm_time()
                } else {
                    self.clock.now()
                };
                self.show_time(&mut v, shown);
                v.colon = alarm_view || !matches!(self.mode, Mode::Clock) || self.clock.colon();
                v.alarm_icon = self.clock.alarm_on;
            }
            Mode::Intro { variant, .. } => {
                show_number(&mut v, self.hiscore % 1000);
                v.game_a = *variant == Variant::A;
                v.game_b = *variant == Variant::B;
            }
            Mode::Play => {
                let r = &self.round;
                show_number(&mut v, r.score());
                v.game_a = r.variant == Variant::A;
                v.game_b = r.variant == Variant::B;
                let full = (r.misses_half / 2) as usize;
                for (i, m) in v.misses.iter_mut().enumerate() {
                    *m = i < full || (i == full && r.misses_half % 2 == 1 && blink_on);
                }
                if r.misses_half >= GAME_OVER_HALVES {
                    v.misses = [true; 3];
                }
                if r.is_over() {
                    v.wolf = Some(r.wolf);
                }
            }
        }
        v
    }

    fn show_time(&self, v: &mut View, t: Shown) {
        v.one = t.hour >= 10;
        v.digits = [Some(t.hour % 10), Some(t.minute / 10), Some(t.minute % 10)];
        v.am = !t.pm;
        v.pm = t.pm;
    }
}

/// Right-aligned score without leading zeros.
fn show_number(v: &mut View, n: u32) {
    let d = [n / 100 % 10, n / 10 % 10, n % 10];
    v.digits = [
        (n >= 100).then_some(d[0] as u8),
        (n >= 10).then_some(d[1] as u8),
        Some(d[2] as u8),
    ];
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Test randomness: a constant, or a small LCG.
    pub struct Seq(Option<f32>, u64);

    impl Seq {
        pub fn never() -> Self {
            Seq(Some(0.999), 0)
        }
        pub fn lcg(seed: u64) -> Self {
            Seq(None, seed)
        }
    }

    impl Random for Seq {
        fn next_f32(&mut self) -> f32 {
            if let Some(v) = self.0 {
                return v;
            }
            self.1 = self
                .1
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.1 >> 40) as f32 / (1u64 << 24) as f32
        }
    }

    fn press(f: impl FnOnce(&mut Inputs)) -> Inputs {
        let mut i = Inputs::default();
        f(&mut i);
        i
    }

    fn start(m: &mut Machine, a: bool) {
        let mut rng = Seq::never();
        m.update(0.0, &press(|i| i.game_a = a), &mut rng);
        m.update(0.0, &press(|i| i.game_b = !a), &mut rng);
        m.update(INTRO_TIME + 0.01, &Inputs::default(), &mut rng);
    }

    #[test]
    fn clock_shows_time() {
        let m = Machine::new(13 * 3600 + 7 * 60, 0);
        let v = m.view(&Inputs::default());
        assert!(!v.one);
        assert_eq!(v.digits, [Some(1), Some(0), Some(7)]);
        assert!(v.pm && !v.am);
    }

    #[test]
    fn holding_time_shows_alarm() {
        let mut m = Machine::new(0, 0);
        m.clock.alarm = 10 * 60 + 30;
        let v = m.view(&press(|i| i.time_held = true));
        assert!(v.one);
        assert_eq!(v.digits, [Some(0), Some(3), Some(0)]);
    }

    #[test]
    fn intro_shows_hiscore_then_plays() {
        let mut m = Machine::new(0, 123);
        let mut rng = Seq::never();
        m.update(0.0, &press(|i| i.game_b = true), &mut rng);
        let v = m.view(&Inputs::default());
        assert_eq!(v.digits, [Some(1), Some(2), Some(3)]);
        assert!(v.game_b);
        m.update(INTRO_TIME + 0.01, &Inputs::default(), &mut rng);
        assert!(m.is_playing());
        assert_eq!(m.view(&Inputs::default()).digits, [None, None, Some(0)]);
    }

    #[test]
    fn half_miss_blinks() {
        let mut m = Machine::new(0, 0);
        start(&mut m, true);
        m.round.misses_half = 3;
        m.blink = 0.1;
        assert_eq!(m.view(&Inputs::default()).misses, [true, true, false]);
        m.blink = 0.6;
        assert_eq!(m.view(&Inputs::default()).misses, [true, false, false]);
    }

    #[test]
    fn game_over_saves_hiscore() {
        let mut m = Machine::new(0, 5);
        start(&mut m, false);
        m.round.total = 42;
        m.round.misses_half = 4;
        m.round.wolf = Ramp::TopLeft;
        m.round.eggs[Ramp::TopRight.index()][POSITIONS - 1] = true;
        m.round.step(&mut Seq::never(), &mut Vec::new());
        m.update(game::MISS_TIME + 0.1, &Inputs::default(), &mut Seq::never());
        assert!(m.round.is_over());
        assert_eq!(m.hiscore, 42);
        assert!(m.hiscore_dirty);
    }

    #[test]
    fn time_button_returns_to_clock() {
        let mut m = Machine::new(0, 0);
        start(&mut m, true);
        m.update(0.0, &press(|i| i.time = true), &mut Seq::never());
        assert!(matches!(m.mode, Mode::Clock));
        assert!(m.round.demo);
    }

    #[test]
    fn set_time_buttons() {
        let mut m = Machine::new(3600, 0);
        let mut rng = Seq::never();
        m.update(0.0, &press(|i| i.time_set = true), &mut rng);
        m.update(0.0, &press(|i| i.ramp = Some(Ramp::TopRight)), &mut rng);
        m.update(0.0, &press(|i| i.ramp = Some(Ramp::BottomRight)), &mut rng);
        m.update(0.0, &press(|i| i.ramp = Some(Ramp::TopLeft)), &mut rng);
        m.update(0.0, &press(|i| i.time_set = true), &mut rng);
        assert!(matches!(m.mode, Mode::Clock));
        assert_eq!(m.clock.minutes(), 14 * 60 + 1);
    }

    #[test]
    fn set_alarm_and_toggle() {
        let mut m = Machine::new(0, 0);
        let mut rng = Seq::never();
        m.update(0.0, &press(|i| i.alarm_set = true), &mut rng);
        m.update(0.0, &press(|i| i.game_a = true), &mut rng);
        assert!(m.clock.alarm_on);
        assert!(matches!(m.mode, Mode::SetAlarm));
        assert!(m.view(&Inputs::default()).alarm_icon);
    }

    #[test]
    fn ringing_alarm_shows_hare_and_stops() {
        let mut m = Machine::new(7 * 3600 - 1, 0);
        m.clock.alarm = 7 * 60;
        m.clock.alarm_on = true;
        let mut rng = Seq::never();
        let cues = m.update(1.5, &Inputs::default(), &mut rng);
        assert!(cues.contains(&Cue::Alarm));
        let v = m.view(&Inputs::default());
        assert!(v.hare && v.bell.is_some());
        m.update(0.0, &press(|i| i.time = true), &mut rng);
        assert!(!m.clock.is_ringing());
    }

    #[test]
    fn demo_plays_itself() {
        let mut m = Machine::new(0, 0);
        let mut rng = Seq::lcg(1);
        for _ in 0..(60 * 120) {
            m.update(1.0 / 60.0, &Inputs::default(), &mut rng);
        }
        assert!(m.round.total > 20);
        assert_eq!(m.hiscore, 0);
    }
}

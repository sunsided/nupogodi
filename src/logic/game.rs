//! One round of egg catching (Game A, Game B, or the clock-mode demo).

use super::{Cue, Ramp, Random, Side, Variant, speed};

/// Egg positions per ramp, top to bottom.
pub const POSITIONS: usize = 5;
/// Misses are counted in halves; this many ends the game (three full misses).
pub const GAME_OVER_HALVES: u8 = 6;
/// Scores (within each thousand) that clear all misses.
pub const CLEAR_SCORES: [u32; 2] = [200, 500];

/// How long the hare stays hidden, then visible, in seconds.
pub const HARE_HIDDEN: f32 = 13.0;
pub const HARE_SHOWN: f32 = 5.0;

/// Miss sequence timing, in seconds.
pub const FALL_TIME: f32 = 0.35;
pub const CHICK_STEP: f32 = 0.35;
pub const MISS_TIME: f32 = 2.0;

/// How a dropped egg ends up on the floor.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MissKind {
    /// No hare: the egg breaks, a full miss.
    Broken,
    /// Hare in the window: a chick hatches and runs off, half a miss.
    Chick,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Phase {
    Playing,
    Miss { side: Side, kind: MissKind, t: f32 },
    Over,
}

/// What the miss sequence currently shows on one side of the floor.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FloorView {
    pub falling: bool,
    pub broken: bool,
    pub hatched: bool,
    /// Running chick position, 0 nearest the egg, 2 at the screen edge.
    pub chick: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Round {
    pub variant: Variant,
    /// Clock-mode attract demo: the wolf plays itself and the round never ends.
    pub demo: bool,
    pub eggs: [[bool; POSITIONS]; 4],
    pub wolf: Ramp,
    /// Caught eggs, not wrapped at 1000, so the pace keeps rising.
    pub total: u32,
    pub misses_half: u8,
    pub hare: bool,
    hare_timer: f32,
    step_timer: f32,
    steps_since_spawn: u32,
    pub phase: Phase,
}

impl Round {
    pub fn new(variant: Variant, demo: bool) -> Self {
        Self {
            variant,
            demo,
            eggs: [[false; POSITIONS]; 4],
            wolf: Ramp::TopLeft,
            total: 0,
            misses_half: 0,
            hare: false,
            hare_timer: 0.0,
            step_timer: 0.0,
            steps_since_spawn: u32::MAX / 2,
            phase: Phase::Playing,
        }
    }

    /// Score as shown on the three-digit display.
    pub fn score(&self) -> u32 {
        self.total % 1000
    }

    pub fn is_over(&self) -> bool {
        self.phase == Phase::Over
    }

    pub fn egg_count(&self) -> usize {
        self.eggs.iter().flatten().filter(|&&e| e).count()
    }

    /// Game A rolls eggs down three ramps; which one is idle depends on the misses so far.
    pub fn idle_ramp(&self) -> Option<Ramp> {
        match self.variant {
            Variant::B => None,
            Variant::A => Some(match self.misses_half {
                0 => Ramp::BottomLeft,
                1 | 2 => Ramp::BottomRight,
                3 | 4 => Ramp::TopLeft,
                _ => Ramp::TopRight,
            }),
        }
    }

    pub fn move_wolf(&mut self, ramp: Ramp) {
        if self.phase != Phase::Over {
            self.wolf = ramp;
        }
    }

    pub fn update(&mut self, dt: f32, rng: &mut dyn Random, cues: &mut Vec<Cue>) {
        match self.phase {
            Phase::Over => return,
            Phase::Miss { side, kind, t } => {
                let t = t + dt;
                if t >= MISS_TIME {
                    self.phase = if self.misses_half >= GAME_OVER_HALVES {
                        if self.demo {
                            self.misses_half = 0;
                            Phase::Playing
                        } else {
                            cues.push(Cue::GameOver);
                            Phase::Over
                        }
                    } else {
                        Phase::Playing
                    };
                    self.step_timer = 0.0;
                } else {
                    self.phase = Phase::Miss { side, kind, t };
                }
                return;
            }
            Phase::Playing => {}
        }

        self.update_hare(dt);

        self.step_timer += dt;
        let interval = speed::step_interval(self.variant, self.total);
        if self.step_timer >= interval {
            self.step_timer -= interval;
            self.step(rng, cues);
        }
    }

    fn update_hare(&mut self, dt: f32) {
        self.hare_timer += dt;
        let limit = if self.hare { HARE_SHOWN } else { HARE_HIDDEN };
        if self.hare_timer >= limit {
            self.hare_timer -= limit;
            self.hare = !self.hare;
        }
    }

    /// Advance every egg one position, resolve arrivals, maybe spawn a new egg.
    pub fn step(&mut self, rng: &mut dyn Random, cues: &mut Vec<Cue>) {
        if self.demo {
            self.autoplay(rng);
        }

        let moved = self.egg_count() > 0;
        for ramp in Ramp::ALL {
            let row = &mut self.eggs[ramp.index()];
            if row[POSITIONS - 1] {
                row[POSITIONS - 1] = false;
                if self.wolf == ramp {
                    self.catch(cues);
                } else {
                    self.miss(ramp, cues);
                    return;
                }
            }
        }
        for row in &mut self.eggs {
            row.rotate_right(1);
        }
        if moved {
            cues.push(Cue::Step);
        }

        self.steps_since_spawn = self.steps_since_spawn.saturating_add(1);
        self.maybe_spawn(rng);
    }

    fn catch(&mut self, cues: &mut Vec<Cue>) {
        self.total += 1;
        cues.push(Cue::Catch);
        if CLEAR_SCORES.contains(&(self.total % 1000)) && self.misses_half > 0 {
            self.misses_half = 0;
            cues.push(Cue::Clear);
        }
    }

    fn miss(&mut self, ramp: Ramp, cues: &mut Vec<Cue>) {
        let kind = if self.hare {
            MissKind::Chick
        } else {
            MissKind::Broken
        };
        self.misses_half += match kind {
            MissKind::Chick => 1,
            MissKind::Broken => 2,
        };
        self.misses_half = self.misses_half.min(GAME_OVER_HALVES);
        self.eggs = [[false; POSITIONS]; 4];
        self.phase = Phase::Miss {
            side: ramp.side(),
            kind,
            t: 0.0,
        };
        cues.push(Cue::Miss);
    }

    fn maybe_spawn(&mut self, rng: &mut dyn Random) {
        if self.steps_since_spawn < speed::min_spawn_gap(self.total)
            || self.egg_count() >= speed::max_eggs(self.variant, self.total)
            || rng.next_f32() >= speed::spawn_chance(self.total)
        {
            return;
        }
        let idle = self.idle_ramp();
        let active: Vec<Ramp> = Ramp::ALL.into_iter().filter(|&r| Some(r) != idle).collect();
        let ramp = active[((rng.next_f32() * active.len() as f32) as usize).min(active.len() - 1)];
        self.eggs[ramp.index()][0] = true;
        self.steps_since_spawn = 0;
    }

    /// Demo wolf: head for the egg closest to the bottom, occasionally guessing wrong.
    fn autoplay(&mut self, rng: &mut dyn Random) {
        let Some(target) = (0..POSITIONS).rev().find_map(|pos| {
            Ramp::ALL
                .into_iter()
                .find(|r| self.eggs[r.index()][pos])
                .map(|r| (r, pos))
        }) else {
            return;
        };
        let (ramp, pos) = target;
        if pos == POSITIONS - 1 && rng.next_f32() < 0.06 {
            self.wolf = Ramp::ALL[(ramp.index() + 1) % 4];
        } else {
            self.wolf = ramp;
        }
    }

    /// Floor segments for one side during a miss sequence.
    pub fn floor(&self, which: Side) -> FloorView {
        let Phase::Miss { side, kind, t } = self.phase else {
            return FloorView::default();
        };
        if side != which {
            return FloorView::default();
        }
        if t < FALL_TIME {
            return FloorView {
                falling: true,
                ..Default::default()
            };
        }
        match kind {
            MissKind::Broken => FloorView {
                broken: true,
                ..Default::default()
            },
            MissKind::Chick => {
                let k = ((t - FALL_TIME) / CHICK_STEP) as u32;
                match k {
                    0 => FloorView {
                        hatched: true,
                        ..Default::default()
                    },
                    1..=3 => FloorView {
                        chick: Some((k - 1) as u8),
                        ..Default::default()
                    },
                    _ => FloorView::default(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::tests::Seq;

    fn round(variant: Variant) -> Round {
        Round::new(variant, false)
    }

    /// Place an egg at the last position of `ramp` and step once.
    fn arrive(r: &mut Round, ramp: Ramp) -> Vec<Cue> {
        let mut cues = Vec::new();
        r.eggs = [[false; POSITIONS]; 4];
        r.eggs[ramp.index()][POSITIONS - 1] = true;
        r.step(&mut Seq::never(), &mut cues);
        cues
    }

    fn finish_miss(r: &mut Round) -> Vec<Cue> {
        let mut cues = Vec::new();
        r.update(MISS_TIME + 0.01, &mut Seq::never(), &mut cues);
        cues
    }

    #[test]
    fn eggs_roll_down() {
        let mut r = round(Variant::B);
        r.eggs[0][0] = true;
        let mut cues = Vec::new();
        r.step(&mut Seq::never(), &mut cues);
        assert!(r.eggs[0][1] && !r.eggs[0][0]);
        assert_eq!(cues, vec![Cue::Step]);
    }

    #[test]
    fn catch_scores() {
        let mut r = round(Variant::B);
        r.wolf = Ramp::BottomRight;
        let cues = arrive(&mut r, Ramp::BottomRight);
        assert_eq!(r.total, 1);
        assert!(cues.contains(&Cue::Catch));
        assert_eq!(r.phase, Phase::Playing);
    }

    #[test]
    fn miss_without_hare_is_full() {
        let mut r = round(Variant::B);
        r.wolf = Ramp::TopLeft;
        let cues = arrive(&mut r, Ramp::TopRight);
        assert_eq!(r.misses_half, 2);
        assert!(cues.contains(&Cue::Miss));
        assert!(matches!(
            r.phase,
            Phase::Miss {
                side: Side::Right,
                kind: MissKind::Broken,
                ..
            }
        ));
        assert_eq!(r.egg_count(), 0);
    }

    #[test]
    fn miss_with_hare_is_half() {
        let mut r = round(Variant::B);
        r.hare = true;
        arrive(&mut r, Ramp::BottomLeft);
        assert_eq!(r.misses_half, 1);
        assert!(matches!(
            r.phase,
            Phase::Miss {
                kind: MissKind::Chick,
                ..
            }
        ));
    }

    #[test]
    fn three_full_misses_end_the_game() {
        let mut r = round(Variant::B);
        r.wolf = Ramp::TopLeft;
        for i in 0..3 {
            arrive(&mut r, Ramp::BottomRight);
            let cues = finish_miss(&mut r);
            assert_eq!(cues.contains(&Cue::GameOver), i == 2);
        }
        assert!(r.is_over());
    }

    #[test]
    fn two_and_a_half_misses_play_on() {
        let mut r = round(Variant::B);
        r.misses_half = 4;
        r.hare = true;
        arrive(&mut r, Ramp::BottomRight);
        finish_miss(&mut r);
        assert_eq!(r.misses_half, 5);
        assert_eq!(r.phase, Phase::Playing);
    }

    #[test]
    fn demo_never_ends() {
        let mut r = Round::new(Variant::A, true);
        r.misses_half = 4;
        r.wolf = Ramp::TopLeft;
        r.eggs[Ramp::BottomRight.index()][POSITIONS - 1] = true;
        // Force the wrong guess path off: autoplay moves to the egg, so make it miss by hand.
        r.miss(Ramp::BottomRight, &mut Vec::new());
        finish_miss(&mut r);
        assert_eq!(r.phase, Phase::Playing);
        assert_eq!(r.misses_half, 0);
    }

    #[test]
    fn score_200_and_500_clear_misses() {
        for s in [199, 499, 1199] {
            let mut r = round(Variant::B);
            r.total = s;
            r.misses_half = 3;
            r.wolf = Ramp::TopLeft;
            let cues = arrive(&mut r, Ramp::TopLeft);
            assert_eq!(r.misses_half, 0, "at {s}");
            assert!(cues.contains(&Cue::Clear));
        }
    }

    #[test]
    fn score_wraps_after_999() {
        let mut r = round(Variant::B);
        r.total = 999;
        r.wolf = Ramp::TopLeft;
        arrive(&mut r, Ramp::TopLeft);
        assert_eq!(r.score(), 0);
        assert_eq!(r.total, 1000);
    }

    #[test]
    fn game_a_idle_ramp_follows_misses() {
        let mut r = round(Variant::A);
        let expect = [
            (0, Ramp::BottomLeft),
            (1, Ramp::BottomRight),
            (2, Ramp::BottomRight),
            (3, Ramp::TopLeft),
            (4, Ramp::TopLeft),
            (5, Ramp::TopRight),
        ];
        for (m, ramp) in expect {
            r.misses_half = m;
            assert_eq!(r.idle_ramp(), Some(ramp));
        }
        assert_eq!(round(Variant::B).idle_ramp(), None);
    }

    #[test]
    fn game_a_never_spawns_on_idle_ramp() {
        let mut r = round(Variant::A);
        let mut rng = Seq::lcg(7);
        let mut cues = Vec::new();
        for _ in 0..2000 {
            r.wolf = Ramp::ALL
                .into_iter()
                .find(|x| r.eggs[x.index()][POSITIONS - 1])
                .unwrap_or(r.wolf);
            r.step(&mut rng, &mut cues);
            assert!(!r.eggs[Ramp::BottomLeft.index()][0]);
        }
        assert!(r.total > 100);
    }

    #[test]
    fn eggs_never_arrive_together() {
        let mut r = round(Variant::B);
        r.total = 300;
        let mut rng = Seq::lcg(3);
        let mut cues = Vec::new();
        for _ in 0..2000 {
            let column = (0..4).filter(|&i| r.eggs[i][POSITIONS - 1]).count();
            assert!(column <= 1);
            r.wolf = Ramp::ALL
                .into_iter()
                .find(|x| r.eggs[x.index()][POSITIONS - 1])
                .unwrap_or(r.wolf);
            r.step(&mut rng, &mut cues);
        }
    }

    #[test]
    fn hare_toggles() {
        let mut r = round(Variant::A);
        let mut cues = Vec::new();
        r.update_hare(HARE_HIDDEN + 0.01);
        assert!(r.hare);
        r.update_hare(HARE_SHOWN);
        assert!(!r.hare);
        r.update(0.0, &mut Seq::never(), &mut cues);
    }

    #[test]
    fn chick_runs_off() {
        let mut r = round(Variant::B);
        r.hare = true;
        arrive(&mut r, Ramp::BottomLeft);
        let at = |r: &mut Round, t: f32| {
            if let Phase::Miss { side, kind, .. } = r.phase {
                r.phase = Phase::Miss { side, kind, t };
            }
            r.floor(Side::Left)
        };
        assert!(at(&mut r, 0.1).falling);
        assert!(at(&mut r, FALL_TIME + 0.1).hatched);
        assert_eq!(at(&mut r, FALL_TIME + CHICK_STEP + 0.1).chick, Some(0));
        assert_eq!(
            at(&mut r, FALL_TIME + 3.0 * CHICK_STEP + 0.1).chick,
            Some(2)
        );
        assert_eq!(r.floor(Side::Right), FloorView::default());
    }
}

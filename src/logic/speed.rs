//! Pacing: how fast eggs roll and how many are on the ramps at once.
//!
//! The original ROM's timing tables are not published, so these values are tuned by feel,
//! seeded from fan reimplementations (0.68 s per step at the start, 0.22 s at the fastest).
//! Everything that controls difficulty lives here.

use super::Variant;

/// Step interval at the very start of Game A, in seconds.
pub const START_INTERVAL_A: f32 = 0.68;
/// Step interval at the very start of Game B (it starts faster).
pub const START_INTERVAL_B: f32 = 0.56;
/// The fastest the eggs ever roll.
pub const MIN_INTERVAL: f32 = 0.22;
/// Effective level at which the minimum interval is reached.
const LEVEL_AT_MIN: f32 = 180.0;
/// After each hundred the game slows down for this many points before climbing again.
pub const HUNDRED_DIP: u32 = 22;
/// How much harder each completed hundred makes the base pace.
const LEVEL_PER_HUNDRED: f32 = 20.0;

/// Difficulty level for a total (unwrapped) score: climbs with the score, drops back for a
/// short stretch after each hundred, and keeps a bit of the progress from earlier hundreds.
pub fn level(total: u32) -> f32 {
    let hundreds = total / 100;
    let within = total % 100;
    let base = hundreds as f32 * LEVEL_PER_HUNDRED;
    if hundreds > 0 && within < HUNDRED_DIP {
        base + within as f32 * 0.5
    } else {
        base + within as f32
    }
}

/// Seconds between two egg steps.
pub fn step_interval(variant: Variant, total: u32) -> f32 {
    let start = match variant {
        Variant::A => START_INTERVAL_A,
        Variant::B => START_INTERVAL_B,
    };
    let t = (level(total) / LEVEL_AT_MIN).min(1.0);
    start + (MIN_INTERVAL - start) * t
}

/// Most eggs allowed on the ramps at once.
pub fn max_eggs(variant: Variant, total: u32) -> usize {
    let lvl = level(total);
    let extra = match variant {
        Variant::A => 0,
        Variant::B => 1,
    };
    (2 + extra + (lvl / 30.0) as usize).min(5)
}

/// Fewest steps between two spawns, so eggs never arrive on the same step.
pub fn min_spawn_gap(total: u32) -> u32 {
    if level(total) < 40.0 { 2 } else { 1 }
}

/// Chance to spawn on a step when allowed.
pub fn spawn_chance(total: u32) -> f32 {
    (0.45 + level(total) / 300.0).min(0.85)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds_up_within_first_hundred() {
        let mut prev = step_interval(Variant::A, 0);
        for s in 1..100 {
            let cur = step_interval(Variant::A, s);
            assert!(cur <= prev, "slower at {s}");
            prev = cur;
        }
        assert!(step_interval(Variant::A, 99) < START_INTERVAL_A);
    }

    #[test]
    fn slows_down_after_each_hundred() {
        for h in [100, 200, 300] {
            assert!(step_interval(Variant::A, h) > step_interval(Variant::A, h - 1));
            assert!(
                step_interval(Variant::A, h + HUNDRED_DIP)
                    < step_interval(Variant::A, h + HUNDRED_DIP - 1)
            );
        }
    }

    #[test]
    fn game_b_is_faster_and_clamped() {
        assert!(step_interval(Variant::B, 0) < step_interval(Variant::A, 0));
        assert_eq!(step_interval(Variant::A, 5000), MIN_INTERVAL);
        assert_eq!(step_interval(Variant::B, 5000), MIN_INTERVAL);
    }

    #[test]
    fn more_eggs_later() {
        assert!(max_eggs(Variant::A, 0) < max_eggs(Variant::A, 150));
        assert!(max_eggs(Variant::B, 0) > max_eggs(Variant::A, 0));
    }
}
